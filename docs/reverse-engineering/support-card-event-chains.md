# Support-card event-chain / outing progress

How to turn the snapshot's per-card progress (`Evaluation.get_StoryStep`) into an
`X / Y` "event chain progress" indicator, using the GameTora data catalog
(`data/gametora/`, see [gametora-data.md](../gametora-data.md)).

> Source: thorough analysis of the committed GameTora snapshots (2026-06-03),
> validated across all 512 stat cards + every friend/group card.

> **Bond value is independent of chain progress.** The friendship gauge (`get_Value`,
> 0–100) does **not** track event-chain/outing progress. A maxed (100/100) bond can
> still have an incomplete chain (e.g. `1/3`). Always source the numerator from the
> fired-event history, never infer it from the bond value.

## TL;DR — the max-step (`Y`) model

| Card `type` (support-cards.json) | Bucket | Max steps `Y` |
|---|---|---|
| `speed` `stamina` `power` `guts` `intelligence` | **Stat** | **By rarity: R=0, SR=2, SSR=3** (formula — *not* an event count) |
| `friend` | **Pal** | size of the **last** event-group in `training-events-friend.json`, keyed by **`char_id`** (≈5; 3 for Sasami) |
| `group` | **Group** | size of the **last** event-group in `training-events-group.json`, keyed by **`support_id`** (5–7) |

`type` and `rarity` (1=R, 2=SR, 3=SSR) both come from
`gametora_data::support_card(id)`.

## Data linkage (the gotchas)

- **Stat cards** — do **not** count the event tree. Use the rarity formula. The
  per-card event count is correct for 501/512 cards but lags/breaks for 11 SSR
  cards (promo / `story_event` / newest gacha — e.g. Satono Diamond promo `30221`,
  Blast Onepiece `30303`), which report 0–1 events despite being SSR. The user-given
  rule (R=0/SR=2/SSR=3) is authoritative and robust.
- **Pal (`friend`) cards** — the friend event file is keyed by **`char_id`, not
  `support_id`**. Tazuna `30021` → `char_id 9001`; Riko `30036` → `char_id 9006`.
  All R/SR/SSR versions of one character share the same friend entry, so Pal outing
  count is per-character and rarity-independent (R Tazuna and SSR Tazuna both = 5).
- **Group cards** — keyed by `support_id` directly. Entry shape is
  `[support_id, [memberCharaIds...], [group0 events], [group1 events]]`. The **last**
  group is the outings (Sirius `30081` = 7, Throne `30067` = 5, matching in-game).

## Event-tree shapes (for reference)

- `training-events-{ssr,sr}.json`: `[[cardId, [event, ...]], ...]` — one flat event
  list; its length is the chain length (SSR=3, SR=2). R cards are simply absent.
- `training-events-friend.json`: `[[charId, [group0], [group1], [group2]], ...]`.
  group0 (~6) = date/training events, group1 (1) = a special event, **group2 (last) =
  outings**.
- `training-events-group.json`: `[[supportId, [members], [group0], [group1]], ...]`.
  group0 = scene/date events (tagged `[4,"ft|pd|at|fs|ff|ny"]`), **group1 (last) =
  per-member outings** (tagged `[5, charaId]`).
- An event is `[eventId, [choices], eventStringId, optional[tagType, payload]]`.
  Tag types seen: `9`=anniversary variant, `1`=region/global flag, `2`=base-card
  reference, `4`=scene-type code, `5`=member-outing.
- `training-events-shared.json` is keyed by `char_id` (131 entries, ~2 events each)
  and is a **separate shared pool**, not part of any per-card chain length.

## Proposed accessor (plugin side)

```rust
// gametora_data.rs
fn last_group_len(tree: &Value, key: i64, key_index_in_entry: usize) -> Option<usize>;

pub fn max_chain_steps(support_id: i64) -> Option<u32> {
    let c = support_card(support_id)?;
    match c.r#type.as_deref()? {
        "speed"|"stamina"|"power"|"guts"|"intelligence" =>
            Some(match c.rarity { Some(3)=>3, Some(2)=>2, _=>0 }),
        "friend" => last_group_len(&training_events(Friend)?, c.char_id?, 0)
                        .map(|n| n as u32),         // keyed by char_id
        "group"  => last_group_len(&training_events(Group)?, support_id, 1)
                        .map(|n| n as u32),         // keyed by support_id
        _ => None,
    }
}
```

## ⚠️ Safety: NEVER resolve the card identity on refresh

`Evaluation.get_TargetId()` returns the **deck slot (1–6)** — not the support-card
master id or char id (102/103 are guests/scenario). Mapping slot → `support_id`
therefore needs the equip array, **but**:

> A prior attempt called `EquipSupportCard.ConvertWorkSupportCardData()` on every
> overlay refresh (~2 s) to resolve names. That API **mutates live equip/support
> state** — it corrupted friendship/evaluation data so bonds vanished in-game **and
> persisted corrupted to the career profile on disk**, and threw
> `NullReferenceException` at career-start `DialogSingleModeStartConfirmEntry.SetupSupportCard()`.

Rules:
- **Never** call `ConvertWorkSupportCardData` / `get_SupportCard*` / `GetCharaId`
  /`GetRarity`/`GetCommandType` on the live work support-card objects on a refresh
  cadence. The *only* safe convert is the single once-per-career pass in
  `deck_bonuses.rs`.
- On **Global**, `get_CharaId` / `get_MasterSupportCard` don't exist on those types
  anyway; `get_Id` on an equip card returns the slot (1–6), not a master id.
- The real support id is `EquipSupportCard.get_SupportCardId()` — an **ObscuredInt**
  (needs decryption, not yet implemented). If read at all, read it **once per
  career** (e.g. folded into `deck_bonuses` capture), never on refresh.

Consequence: the `X/Y` **denominator** is blocked on a safe slot→support_id mapping
(ObscuredInt decryption, once per career). The **numerator** (`story_step`) is safe
to read every refresh — it lives on the `Evaluation` object we already iterate.

## ⚠️ Still unverified at runtime — the numerator (`X`)

`Evaluation.get_StoryStep()` is the obvious numerator for **stat** cards (it tracks
the 2–3 date-event chain). For **Pal/Group**, the "chain" is the *outings*, and the
`Evaluation` object also exposes `get_IsOuting`, and **`GroupOutingInfoList`** with
per-member steps (UI uses `GetGroupCharaStep` / `GetGroupCharaStepMax`). Open
question to settle on a live career with Pal/Group cards:

> Does `get_StoryStep` increment per **outing** for Pal/Group, or does outing
> progress live only in `GroupOutingInfoList` / `get_IsOuting`?

If the latter, the Pal/Group numerator must come from `GroupOutingInfoList`, not
`StoryStep`. The denominator (this doc) is settled either way.

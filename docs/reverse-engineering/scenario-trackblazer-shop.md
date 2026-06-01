# Trackblazer (Make a New Track) — RaceCoin Shop

Live readout of the Trackblazer scenario's RaceCoin shop, surfaced in the overlay's
**Scenario** tab. First instance of the per-scenario submodule
(`memory_reader/scenario/`), which dispatches structurally on whichever
`WorkSingleModeScenario*` work object is present.

## Access chain

Hangs off the existing chara-data work object (`chain::get_chara_ptr()` →
`WorkSingleModeCharaData`):

```text
WorkSingleModeCharaData.get_WorkScenarioFree() -> Gallop.WorkSingleModeScenarioFree
  (null unless the active scenario is Trackblazer — used as the dispatch test)
```

`WorkSingleModeScenarioFree` getters — all return plain `Int32` / managed refs, so
they are **directly callable** (unlike ObscuredInt getters, which need field decrypt):

| Method | Returns | Meaning |
|--------|---------|---------|
| `get_CoinNum()` | `Int32` | Player RaceCoins (backing field `_coinNum` is ObscuredInt; getter returns decrypted) |
| `get_ShopId()` | `Int32` | Current shop lineup id |
| `get_SaleValue()` | `Int32` | Sale % / discount |
| `get_WinPoints()` | `Int32` | Win points |
| `get_GainedCoinNum()` | `Int32` | Coins gained (cumulative) |
| `get_PickUpItemInfoArray()` | `SingleModeFreePickUpItem[]` | Current shop lineup |
| `get_UserItemInfoArray()` | `SingleModeFreeUserItem[]` | Owned items |
| `get_SingleModeFreeItemEffectArray()` | `SingleModeFreeItemEffect[]` | Active item effects (begin/end turn) |

### Struct fields (plain `Int32`, read directly from the object)

`SingleModeFreePickUpItem`:
`shop_item_id, item_id, coin_num, original_coin_num, item_buy_num, limit_buy_count, limit_turn`

`SingleModeFreeUserItem`: `item_id, num`

`SingleModeFreeItemEffect`:
`use_id, item_id, effect_type, effect_value_1..4, begin_turn, end_turn`

## IL2CPP array layout (64-bit)

Managed reference-type arrays (`T[]`): element count at offset `0x18`, inline
element pointer buffer starts at `0x20`. Read via `il2cpp::read_obj_array` →
`(base, len)`, then `*base.add(i)` per element. Plain `Int32` fields read with
`il2cpp::read_i32_field`.

## Dispatch rationale

We do **not** need the numeric `get_ScenarioId` value to ship: `get_WorkScenarioFree()`
returns null for non-Trackblazer runs, so structural presence is the discriminant.
The existing `scenario_command_base == 1101` (Trackblazer Speed-slot command id) is an
equivalent signal if a non-structural key is ever needed.

## Code

- `memory_reader/scenario/mod.rs` — `ScenarioState` enum + `read_scenario_state(chara)` dispatch.
- `memory_reader/scenario/trackblazer.rs` — `read_shop`, `TrackblazerShop`,
  `TrackblazerShopItem` (+ `discounted()` / `sold_out()` pure helpers, unit-tested).
  On-change diagnostic `Trackblazer shop: coins=… sale=… items(item_id,name,value,owned,bought,limit)=…`
  for in-game cross-check.
- `memory_reader/scenario/master_shop.rs` — main-thread master-data enrichment
  (item name via `MasterString.GetText`, fallback value via the free-shop master tables).
- `memory_reader/scenario/items.rs` — curated `item_id` → (Effect, Worth) catalog
  (from uma.guide), pure + unit-tested.
- `memory_reader/snapshot.rs` — `CareerSnapshot.scenario_state` filled on the
  main-thread refresh.
- `ui.rs` — `Tab::Scenario` + `draw_scenario_tab` / `draw_trackblazer_shop`.

## v2 enrichment — name / value / owned (Hachimi-Edge-bt0)

The Scenario tab renders the shop lineup as a grid **Item | Effect | Price | Avail | Worth**,
with a separate **Owned items** list below (name + effect + ×count). Enrichment runs in
`read_shop` on the Unity main thread (`memory_reader/scenario/master_shop.rs`).

**Avail** = `SingleModeFreePickUpItem.limit_turn` ("N turn(s)" in-game — turns the item stays
in the shop). Surfaced raw as `turns_left`; whether it's remaining vs absolute is pending
in-game confirmation (the in-game readout shows 4/1/1 for the sample lineup).

### Localized item name

`MasterString.Category` has **no literal values in the metadata dump**, so the int for
`SingleModeScenarioFreeItemName` cannot be read offline. It is discovered at runtime:

```text
Singleton<MasterDataManager>
  ._<masterString>k__BackingField -> Gallop.MasterString
      .GetText(Category category /*Int32*/, Int32 index) -> System.String
```

`MasterDataManager` has the `<masterString>k__BackingField` field but **no `get_masterString`
getter** (the getter lives on `MasterSystemDatabase`), so we read the backing field directly.
We probe `category` over `0..600`, calling `GetText(category, item_id)` for every lineup
`item_id`; the category that resolves the most ids to non-empty strings wins (full coverage
short-circuits). The winner is cached in a static and logged once:
`Trackblazer name category discovered: <N> (k/n items resolved)`. `GetText` returns null for
unknown `(category,index)` keys, so probing is side-effect-free.

### Effect + Worth columns

The UI shows a human-readable **Effect** string and an editorial **Worth** tier, both
sourced from a curated catalog keyed by `item_id` (`memory_reader/scenario/items.rs`),
built from <https://uma.guide/guides/trackblazer#coin-shop-and-items>. The shop icon
filename `scenario_free_item_icon_0XXXXX` = `item_id` (e.g. `01201` → 1201), so the
catalog and the in-game `item_id`s should align (verify via the on-change log).

`Worth` ∈ { MustBuy, Situational, Optional, Skip } — drives a future decision picker.

For **unlisted** items we fall back to the raw master value:

```text
MasterDataManager.get_masterSingleModeFreeShopItem()   -> MasterSingleModeFreeShopItem
    .GetWithItemId(Int32 itemId)        -> SingleModeFreeShopItem { EffectGroupId, ... }
MasterDataManager.get_masterSingleModeFreeShopEffect() -> MasterSingleModeFreeShopEffect
    .GetWithEffectGroupId(Int32 grpId)  -> SingleModeFreeShopEffect { EffectType, EffectValue1..4, Turn }
```

→ `EffectValue1` of the first effect in the item's `EffectGroupId`. The raw `effect_type`
int → label mapping is **not yet decoded**, which is why the curated catalog is the
primary Effect source; decoding `effect_type` would let us generate Effect strings for
unlisted/future items from game data directly.

### Owned items (separate list)

`WorkSingleModeScenarioFree.get_UserItemInfoArray()` → `SingleModeFreeUserItem { item_id, num }`,
collected into an `item_id -> num` map and surfaced as `TrackblazerShop.owned`
(`TrackblazerOwnedItem { item_id, name, effect, count }`, sorted by `item_id`, `count > 0`).
Rendered as its own **Owned items** section rather than a per-row shop column, so the shop
lineup (what's for sale) stays distinct from inventory (what you hold).

### Still deferred

- Decode `effect_type` int → label so Effect strings can be generated from game data
  (would cover unlisted/future items without a curated entry).
- Active effect durations via `get_SingleModeFreeItemEffectArray()`
  (`SingleModeFreeItemEffect { begin_turn, end_turn }`).
- Unity Cup / Aoharu reader (`get_TeamRace()` → `WorkSingleModeScenarioTeamRace`).

### Needs in-game verification

- Item names match the in-game shop (and the discovered category int is stable).
- "Value" magnitude matches what the shop tooltip shows (e.g. Empowering Megaphone = 60).
- owned / buy counts match.

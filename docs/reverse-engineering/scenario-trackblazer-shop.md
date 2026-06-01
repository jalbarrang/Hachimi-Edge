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
  On-change diagnostic `Trackblazer shop: coins=… sale=… items(...)=…` for in-game
  cross-check.
- `memory_reader/snapshot.rs` — `CareerSnapshot.scenario_state` filled on the
  main-thread refresh.
- `ui.rs` — `Tab::Scenario` + `draw_scenario_tab` / `draw_trackblazer_shop`.

## Deferred (v2)

- Localized item names via MasterString `SingleModeScenarioFreeItemName` (currently
  shows `#item_id`).
- Effect decoding via `MasterSingleModeFreeShopItem` / `MasterSingleModeFreeShopEffect`.
- Owned items + active effect durations (`UserItemInfoArray`, `SingleModeFreeItemEffectArray`).
- Unity Cup / Aoharu reader (`get_TeamRace()` → `WorkSingleModeScenarioTeamRace`) using
  the same submodule pattern.

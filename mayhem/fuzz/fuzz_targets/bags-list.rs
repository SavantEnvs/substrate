#![no_main]
// Ported from the mayhemheroes honggfuzz driver (frame/bags-list/fuzzer/src/main.rs,
// mayhemheroes/substrate@9ef757989da576b87a6f77c2132ff117f0e6d529): that driver decodes ONE
// `(AccountId, VoteWeight, u32)` tuple per honggfuzz iteration via `arbitrary` (which zero-pads
// missing trailing bytes rather than rejecting a short input — the original run's crashing input
// is 4 bytes, i.e. only `account_id_seed` is present and the rest reads as zero) and applies ONE
// action, but does so against a mock externality built ONCE and kept alive for the entire
// `loop { fuzz!(...) } ` — honggfuzz reuses one process across many iterations, so the pallet
// storage accumulates across actions, and the crash found is a corrupted-list invariant that only
// shows up after many prior actions, not from this one action against fresh genesis state. A
// libFuzzer call is one action too, but per-call reset storage (a fresh `build_and_execute` each
// time, as an earlier port here did) throws that accumulated state away and never reproduces.
// `pallet_bags_list::mock::ExtBuilder::build()` is `pub(crate)` so it can't be called from this
// crate directly; instead this rebuilds the same genesis (System genesis storage + the same
// `GENESIS_IDS` inserted via the public `SortedListProvider::on_insert`, which is exactly what
// `List::insert` — what `build()` calls internally — does for a not-yet-present id) once per
// process, in a `thread_local`, and reuses it across every subsequent libFuzzer call on this
// thread.
use frame_election_provider_support::{ScoreProvider, SortedListProvider};
use libfuzzer_sys::fuzz_target;
use pallet_bags_list::mock::{AccountId, BagsList, Runtime, StakingMock};
use std::cell::RefCell;

const ID_RANGE: AccountId = 25_000;

// Mirrors `pallet_bags_list::mock::GENESIS_IDS` (private to that crate).
const GENESIS_IDS: [(AccountId, u64); 4] = [(1, 10), (2, 1_000), (3, 1_000), (4, 1_000)];

thread_local! {
    static EXT: RefCell<Option<sp_io::TestExternalities>> = RefCell::new(None);
}

enum Action {
    Insert,
    Update,
    Remove,
}

impl From<u32> for Action {
    fn from(v: u32) -> Self {
        let num_variants = Self::Remove as u32 + 1;
        match v % num_variants {
            x if x == Action::Insert as u32 => Action::Insert,
            x if x == Action::Update as u32 => Action::Update,
            _ => Action::Remove,
        }
    }
}

// Zero-pads missing trailing bytes instead of rejecting a short input — matches `arbitrary`'s
// behavior for fixed-width integers, which the original honggfuzz `fuzz!(|data: (...)| ...)`
// relied on.
fn take_u32(data: &[u8], pos: &mut usize) -> u32 {
    let mut buf = [0u8; 4];
    let n = data.len().saturating_sub(*pos).min(4);
    buf[..n].copy_from_slice(&data[*pos..*pos + n]);
    *pos += n;
    u32::from_le_bytes(buf)
}

fn take_u64(data: &[u8], pos: &mut usize) -> u64 {
    let mut buf = [0u8; 8];
    let n = data.len().saturating_sub(*pos).min(8);
    buf[..n].copy_from_slice(&data[*pos..*pos + n]);
    *pos += n;
    u64::from_le_bytes(buf)
}

fuzz_target!(|data: &[u8]| {
    let mut pos = 0;
    let account_id_seed = take_u32(data, &mut pos);
    let vote_weight = take_u64(data, &mut pos);
    let action_seed = take_u32(data, &mut pos);

    let id = account_id_seed % ID_RANGE;
    let action = Action::from(action_seed);

    EXT.with(|cell| {
        let mut slot = cell.borrow_mut();
        let ext = slot.get_or_insert_with(|| {
            let storage = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();
            let mut ext = sp_io::TestExternalities::from(storage);
            ext.execute_with(|| {
                for (genesis_id, weight) in GENESIS_IDS {
                    BagsList::on_insert(genesis_id, weight).unwrap();
                    StakingMock::set_score_of(&genesis_id, weight);
                }
            });
            ext
        });

        ext.execute_with(|| {
            match action {
                Action::Insert => {
                    if BagsList::on_insert(id, vote_weight).is_err() {
                        // this was a duplicate id, which is ok. We can just update it.
                        BagsList::on_update(&id, vote_weight).unwrap();
                    }
                    assert!(BagsList::contains(&id));
                }
                Action::Update => {
                    let already_contains = BagsList::contains(&id);
                    if already_contains {
                        BagsList::on_update(&id, vote_weight).unwrap();
                        assert!(BagsList::contains(&id));
                    } else {
                        BagsList::on_update(&id, vote_weight).unwrap_err();
                    }
                }
                Action::Remove => {
                    let already_contains = BagsList::contains(&id);
                    if already_contains {
                        BagsList::on_remove(&id).unwrap();
                    } else {
                        BagsList::on_remove(&id).unwrap_err();
                    }
                    assert!(!BagsList::contains(&id));
                }
            }

            assert!(BagsList::sanity_check().is_ok());
        });
    });
});

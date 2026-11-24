pub struct MockEnv;

pub struct MockLedger;

impl MockEnv {
    pub fn ledger(&self) -> MockLedger {
        MockLedger
    }
}

impl MockLedger {
    pub fn sequence(&self) -> u32 {
        0
    }

    pub fn timestamp(&self) -> u64 {
        0
    }

    pub fn network_id(&self) -> [u8; 32] {
        [0u8; 32]
    }

    pub fn protocol_version(&self) -> u32 {
        0
    }

    pub fn max_live_until_ledger(&self) -> u32 {
        0
    }
}

fn mock_env_in_for_loop(mock_env: MockEnv) {
    for _ in 0..10 {
        let _seq = mock_env.ledger().sequence();
        let _ts = mock_env.ledger().timestamp();
        let _nid = mock_env.ledger().network_id();
        let _proto = mock_env.ledger().protocol_version();
        let _max = mock_env.ledger().max_live_until_ledger();
    }
}

fn mock_env_in_while_loop(mock_env: MockEnv) {
    let mut i = 0;
    while i < 10 {
        let _seq = mock_env.ledger().sequence();
        i += 1;
    }
}

fn mock_env_in_closure(mock_env: MockEnv) {
    (0..5).for_each(|_| {
        let _seq = mock_env.ledger().sequence();
    });
}

fn bound_mock_ledger_in_loop(mock_env: MockEnv) {
    let ledger = mock_env.ledger();
    for _ in 0..10 {
        let _seq = ledger.sequence();
        let _ts = ledger.timestamp();
    }
}

fn main() {}

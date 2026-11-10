use super::*;

const MODELS: [Model; 3] = [Model::Eb3a, Model::Ac180, Model::Elite200V2];

fn pairs(model: Model) -> Vec<(Output, u16, u16)> {
    switches(model)
        .iter()
        .map(|s| (s.output(), s.register(), s.read_back()))
        .collect()
}

#[test]
fn eb3a_writes_3007_3008_and_reads_back_48_49() {
    assert_eq!(
        pairs(Model::Eb3a),
        [(Output::Ac, 3007, 48), (Output::Dc, 3008, 49)]
    );
}

#[test]
fn ac180_writes_and_reads_back_2011_2012() {
    assert_eq!(
        pairs(Model::Ac180),
        [(Output::Ac, 2011, 2011), (Output::Dc, 2012, 2012)]
    );
}

#[test]
fn elite_has_no_switch_until_h16() {
    assert!(switches(Model::Elite200V2).is_empty());
}

#[test]
fn every_switch_is_unverified() {
    for model in MODELS {
        assert!(switches(model).iter().all(|s| !s.verified()), "{model:?}");
    }
}

#[test]
fn no_register_outside_the_allow_list() {
    for model in MODELS {
        for s in switches(model) {
            assert!(
                [3007, 3008, 2011, 2012].contains(&s.register()),
                "{model:?}"
            );
        }
    }
}

#[test]
fn output_keys_round_trip() {
    for output in [Output::Ac, Output::Dc] {
        assert_eq!(Output::from_key(output.key()), Some(output));
    }
    assert_eq!(Output::from_key("AC"), None);
    assert_eq!(Output::from_key("usb"), None);
}

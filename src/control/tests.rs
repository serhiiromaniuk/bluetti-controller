use super::*;

fn station(model: Model) -> StationConfig {
    StationConfig {
        name: "router".into(),
        model,
        address: "AA:BB:CC:00:00:00".into(),
        grid_sensor: false,
    }
}

const ENABLED: ControlConfig = ControlConfig { enabled: true };

#[test]
fn disabled_refuses_every_station_and_output() {
    let disabled = ControlConfig::default();
    for model in [Model::Eb3a, Model::Ac180, Model::Elite200V2] {
        for output in [Output::Ac, Output::Dc] {
            for on in [true, false] {
                let refused = Command::new(&disabled, &station(model), output, on).unwrap_err();
                assert_eq!(refused, Refused::Disabled);
            }
        }
    }
}

#[test]
fn enabled_builds_with_the_listed_switch() {
    let command = Command::new(&ENABLED, &station(Model::Eb3a), Output::Dc, false).unwrap();
    assert_eq!(command.station(), "router");
    assert_eq!(command.output(), Output::Dc);
    assert!(!command.on());
    assert_eq!(command.switch().register(), 3008);
    assert_eq!(command.switch().read_back(), 49);

    let command = Command::new(&ENABLED, &station(Model::Ac180), Output::Ac, true).unwrap();
    assert!(command.on());
    assert_eq!(command.switch().register(), 2011);
}

#[test]
fn enabled_builds_the_elite_pair() {
    for (output, register) in [(Output::Ac, 2011), (Output::Dc, 2012)] {
        let command = Command::new(&ENABLED, &station(Model::Elite200V2), output, true).unwrap();
        assert_eq!(command.switch().register(), register);
    }
}

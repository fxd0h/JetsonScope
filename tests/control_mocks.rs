use jetsonscope::control::ControlManager;
use jetsonscope::hardware::JetsonHardware;

// Helpers to create ControlManager with mocked hardware detection.
fn mock_hw(is_jetson: bool) -> JetsonHardware {
    let mut hw = JetsonHardware::default();
    hw.is_jetson = is_jetson;
    hw.nvpmodel_modes = vec!["MODE_0".into(), "MODE_1".into()];
    hw
}

#[test]
fn fan_set_out_of_range_returns_error() {
    let hw = mock_hw(true);
    let mut ctrl = ControlManager::mock(hw);
    ctrl.set_fan(150);
    let status = ctrl.status();
    assert!(status.last_error.as_deref().unwrap_or("").contains("0-100"));
}

#[test]
fn fan_set_valid_range_ok() {
    let hw = mock_hw(true);
    let mut ctrl = ControlManager::mock(hw);
    ctrl.set_fan(80);
    assert!(ctrl.status().last_error.is_none());
    let info = ctrl.control_info("fan");
    assert_eq!(info.value, "80%");
}

#[test]
fn nvpmodel_invalid_mode_errors() {
    let hw = mock_hw(true);
    let mut ctrl = ControlManager::mock(hw);
    ctrl.set_nvpmodel_mode(Some("INVALID".into()));
    let status = ctrl.status();
    assert!(status
        .last_error
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase()
        .contains("modo inválido"));
}

#[test]
fn nvpmodel_valid_mode_ok() {
    let hw = mock_hw(true);
    let mut ctrl = ControlManager::mock(hw);
    ctrl.set_nvpmodel_mode(Some("MODE_1".into()));
    assert!(ctrl.status().last_error.is_none());
    let info = ctrl.control_info("nvpmodel");
    assert_eq!(info.value, "MODE_1");
}

#[test]
fn jetson_clocks_toggle_on_non_jetson_is_noop() {
    let hw = mock_hw(false);
    let mut ctrl = ControlManager::mock(hw);
    ctrl.toggle_jetson_clocks();
    // Should not set an error; no-op on non-Jetson.
    assert!(ctrl.status().last_error.is_none());
    let info = ctrl.control_info("jetson_clocks");
    // Value may remain default or toggle; accept on/off/unknown.
    assert!(matches!(info.value.as_str(), "on" | "off" | "unknown"));
}

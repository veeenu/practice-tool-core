use serde::Deserialize;
use windows::Win32::UI::Input::XboxController::*;

#[derive(Default, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
#[serde(try_from = "String")]
pub struct ControllerCombination {
    buttons: XINPUT_GAMEPAD_BUTTON_FLAGS,
    left_trigger: bool,
    right_trigger: bool,
}

impl ControllerCombination {
    pub fn is_pressed(&self, state: &XINPUT_STATE) -> bool {
        state.Gamepad.wButtons.contains(self.buttons)
            && (!self.left_trigger
                || state.Gamepad.bLeftTrigger > XINPUT_GAMEPAD_TRIGGER_THRESHOLD.0 as u8)
            && (!self.right_trigger
                || state.Gamepad.bRightTrigger > XINPUT_GAMEPAD_TRIGGER_THRESHOLD.0 as u8)
    }
}

impl TryFrom<&str> for ControllerCombination {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value = value.to_ascii_lowercase();
        let value = value.split('+');
        let mut combination = ControllerCombination::default();

        for s in value {
            match s {
                "l1" => combination.buttons |= XINPUT_GAMEPAD_LEFT_SHOULDER,
                "l2" => combination.left_trigger = true,
                "l3" => combination.buttons |= XINPUT_GAMEPAD_LEFT_THUMB,
                "r1" => combination.buttons |= XINPUT_GAMEPAD_RIGHT_SHOULDER,
                "r2" => combination.right_trigger = true,
                "r3" => combination.buttons |= XINPUT_GAMEPAD_RIGHT_THUMB,
                "down" => combination.buttons |= XINPUT_GAMEPAD_DPAD_DOWN,
                "up" => combination.buttons |= XINPUT_GAMEPAD_DPAD_UP,
                "right" => combination.buttons |= XINPUT_GAMEPAD_DPAD_RIGHT,
                "left" => combination.buttons |= XINPUT_GAMEPAD_DPAD_LEFT,
                "a" => combination.buttons |= XINPUT_GAMEPAD_A,
                "b" => combination.buttons |= XINPUT_GAMEPAD_B,
                "x" => combination.buttons |= XINPUT_GAMEPAD_X,
                "y" => combination.buttons |= XINPUT_GAMEPAD_Y,
                "back" => combination.buttons |= XINPUT_GAMEPAD_BACK,
                "start" => combination.buttons |= XINPUT_GAMEPAD_START,
                other => return Err(format!("Not a controller button: {other}")),
            }
        }

        Ok(combination)
    }
}

impl TryFrom<String> for ControllerCombination {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.as_str().try_into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let state = XINPUT_STATE {
            dwPacketNumber: 0,
            Gamepad: XINPUT_GAMEPAD {
                wButtons: XINPUT_GAMEPAD_LEFT_THUMB
                    | XINPUT_GAMEPAD_RIGHT_THUMB
                    | XINPUT_GAMEPAD_DPAD_LEFT
                    | XINPUT_GAMEPAD_LEFT_SHOULDER,
                bLeftTrigger: 100,
                bRightTrigger: 100,
                sThumbLX: 0,
                sThumbLY: 0,
                sThumbRX: 0,
                sThumbRY: 0,
            },
        };

        assert!(ControllerCombination::try_from("l3+r3").unwrap().is_pressed(&state));
        assert!(ControllerCombination::try_from("l2+r2").unwrap().is_pressed(&state));
        assert!(ControllerCombination::try_from("l1+left").unwrap().is_pressed(&state));
        assert!(ControllerCombination::try_from("l2+left").unwrap().is_pressed(&state));
        assert!(!ControllerCombination::try_from("l1+r1").unwrap().is_pressed(&state));
    }
}

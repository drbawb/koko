use std::collections::HashMap;

use glium::glutin::VirtualKeyCode as KeyCode;

/// Responds to inquiries regarding three sets of keyboard input.
///
///- Pressed keys
///- Released keys
///- Held keys
#[derive(Default)]
pub struct Input {
    pressed_keys:   HashMap<u32, bool>,
    released_keys:  HashMap<u32, bool>,
    held_keys:      HashMap<u32, bool>,
}

impl Input {
    pub fn new() -> Input {
        Default::default()
    }

    /// Resets the toggle states of pressed & released keys.
    pub fn begin_new_frame(&mut self) {
        self.pressed_keys.clear();
        self.released_keys.clear();
    }

    /// Handles a key down event
    pub fn key_down_event(&mut self, key: KeyCode) {
        self.pressed_keys.insert(key as u32, true);
        self.held_keys.insert(key as u32, true);
    }

    /// Handles a key up event
    pub fn key_up_event(&mut self, key: KeyCode) {
        self.released_keys.insert(key as u32, true);
        self.held_keys.insert(key as u32, false);
    }

    /// Responds true if key was pressed since last call to `beginNewFrame()`.
    /// Responds false otherwise.
    pub fn was_key_pressed(&self, key: KeyCode) -> bool {
        let key_cap = &(key as u32);
        match self.pressed_keys.get(key_cap) {
            Some(is_pressed) => *is_pressed,
            None             => false,
        }
    }
    
    /// Responds true if key was released since last call to `beginNewFrame()`.
    /// Responds false otherwise.
    pub fn was_key_released(&self, key: KeyCode) -> bool {
        let key_cap = &(key as u32);
        match self.released_keys.get(key_cap) {
            Some(is_pressed) => *is_pressed,
            None             => false,
        }
    }
    
    /// Responds true if key has been pressed since last call to `beginNewFrame()`
    /// but _has not yet been released._
    ///
    /// Responds false otherwise.
    pub fn is_key_held(&self, key: KeyCode) -> bool {
        let key_cap = &(key as u32);
        match self.held_keys.get(key_cap) {
            Some(is_pressed) => *is_pressed,
            None             => false,
        }
    }
}

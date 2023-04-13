use godot::engine::{Engine, Node2D, Node2DVirtual};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct MainBehavior {
    #[base]
    base: Base<Node2D>,
}

#[godot_api]
impl Node2DVirtual for MainBehavior {
    fn init(base: Base<Node2D>) -> Self {
        MainBehavior { base }
    }
    fn process(&mut self, _delta: f64) {
        if Engine::singleton().is_editor_hint() {
            return;
        }
        let input = Input::singleton();
        if input.is_action_just_pressed("button_reset".into(), false) {
            self.get_tree().unwrap().reload_current_scene();
        }
    }
}

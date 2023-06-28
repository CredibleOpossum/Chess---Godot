extends Timer

# this is a dirty hack, I will implement a Rust scene switcher to solve this
func _on_timeout():
	self.get_tree().change_scene_to_file("res://scenes/main_chess_scene.tscn")

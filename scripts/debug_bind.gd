extends Timer

func _on_timeout():
	self.get_tree().change_scene_to_file("res://scenes/debug_scene.tscn")

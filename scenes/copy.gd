extends Button

func _on_button_down():
	DisplayServer.clipboard_set($"../ColorRect/RichTextLabel".get_num_string())

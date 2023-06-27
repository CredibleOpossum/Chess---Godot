extends Node

func _on_text_submitted(new_text):
	self.get_node("/root/WaitLogic").try_connect(new_text)
 

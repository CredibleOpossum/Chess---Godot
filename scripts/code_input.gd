extends Node

func _on_text_submitted(new_text):
	var input_num = int(new_text)
	if input_num > 0 && input_num != GenerateAndWait.number:
		self.get_node("/root/WaitLogic/WaitRoom").try_connect(new_text)
 

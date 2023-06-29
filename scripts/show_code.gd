extends RichTextLabel

func get_num_string():
	var string_num = str(GenerateAndWait.number)
	var padding = ""
	for i in 5 - len(string_num):
		padding += "0"
	return padding + string_num;

func _ready():
	self.set_text("[center]" + get_num_string())

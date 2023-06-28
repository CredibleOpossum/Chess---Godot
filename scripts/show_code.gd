extends RichTextLabel

func _ready():
	var string_num = str(GenerateAndWait.number)
	var padding = ""
	for i in 5 - len(string_num):
		padding += "0"
	self.set_text("[center]" + padding + string_num)

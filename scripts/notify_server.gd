extends Node2D

func _ready():
	self.get_node("/root/WaitLogic/WaitRoom").try_connect(str(GenerateAndWait.number))

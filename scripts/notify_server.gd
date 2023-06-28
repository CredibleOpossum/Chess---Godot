extends Node2D

func _ready():
	WaitRoom.try_connect(str(GenerateAndWait.number))

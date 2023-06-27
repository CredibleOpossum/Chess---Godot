extends Node

# Generating on the client could be a little janky, but there shouldn't be any
# practical issues with this
var number = randi_range(0,99999)

func _ready():
		print("client started, id is: " + str(number))
		self.get_node("/root/WaitLogic").spawn_waiting_thread	(str(number))	

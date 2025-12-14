# Look Away

A game with (actual) time travel mechanics. 

## Level Editor Info

The level editor will save and load levels into the `resources/levels` directory found in the same folder it is run from, if found. The game will start at the level called `start`.

Press `F3` or `shift + 0` to toggle the level editor.

You can click and drag entities. Hold shift to snap to the nearest half tile.

Commands: 
* `/save ?name` saves the level to the provided name. Adding characters that could be potentially interpreted as file path delimeters such as `/` or `\` may cause unexpected behavior. No spaces are allowed and anything after the first space will be ignored. If `name` is not provided it will save to the currently loaded level.
* `/load ?name` loads the level with the provided name. As above, don't add spaces, `/`, or `\`. If `name` is not provided it will load the last saved state of the current level.
* `/clear` clears the loaded level without effecting any level files. Using `/save` or `/load` immediately after calling this will require that `name` is specified.
* `/tile ?leftclick ?rightclick ?middleclick` enters tile painting mode. The available tiles are `empty`, `brick1`, `brick2`, `wood`, and `hourglass`. If an argument is not provided, it will default to `empty`. Press `escape` or `/` to exit tile painting mode.
* `/entity (...)` will enter entity placing mode with the entity you specified. Hold `shift` to snap to the nearest half tile. Its subcommands are:
  * `elevator kind direction ?exit_path`. Available kinds are `loop`, `entry`, `exit`, and `inverse_loop` (the broken elevator at the end of the game). If `exit` is specified, `exit_path` must be provided and refers to the destination of the exit elevator. Valid directions are `north`, `south`, `east`, and `west`.
  * `player`. Self explanatory. Should be placed in the center of the `entry` elevator.
  * `gate kind ?direction`. Available kinds are `and`, `or`, `not`, `passthrough`, `toggle`, `toggle_on`, `hold`, `hold_on`, `start`, `end`, `delay`, and `output`. Direction can be any cardinal direction as for the `elevator`, and defaults to `east`.
* `/delete` will enter delete mode. Right click to delete the selected entity.
* `/wire` will enter wire mode. Right or middle click on the input, and right click on the output to add a connection. If you middle click in stead, it will remove a connection if there is one.
* `/shift x y` will move the level by the offset `(x, y)`, where `+x` is right and `+y` is down.

If you provide too many arguments to a command, the extra arguments will be silently ignored.

TODO: Explain the logic gate kinds.

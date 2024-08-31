# wsctrl 

CLI tool to manage workspaces via the [ext-workspace-unstable-v1(2020)](https://gitlab.freedesktop.org/wayland/wayland-protocols/-/merge_requests/40/diffs?commit_id=f017d96d1f71f8e9165365735a0071e4e981e3f6#b449569b3f5835bd6102550cf485143e15025cc9wayland), [ext-workspace-v1](https://gitlab.freedesktop.org/wayland/wayland-protocols/-/merge_requests/40) or [cosmic-workspace-unstable-v1](https://github.com/pop-os/cosmic-protocols/blob/main/unstable/cosmic-workspace-unstable-v1.xml) wayland protocol extension. 

## install & run

```
$ git clone https://github.com/felixgruenbauer/wsctrl.git
$ cd wsctrl
$ cargo run
```

## use

```
$ wsctrl
Manage workspaces via the wayland protocol extension 'ext-workspace-v1'.

Usage: wsctrl <COMMAND>

Commands:
  activate          Activate selected workspace. Some options require an output selection. [aliases: a]
  deactivate        Deactivate selected workspace. Some options require an output selection. [aliases: d]
  assign            Assign workspace to selected output. [aliases: s]
  remove            Remove selected workspace. Some options require an output selection. [aliases: r]
  create-workspace  Create workspace on selected output. [aliases: cw]
  list              List workspaces. Global or on selected output. [aliases: ls]
  help              Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

```
$ wsctrl activate -h
Activate selected workspace. Some options require an output selection.

Usage: wsctrl activate [OPTIONS] <--active|--index <INDEX>|--name <NAME>|--protocol-id <ID>>

Options:
  -h, --help  Print help

Workspace selection (exclusive):
  -a, --active            Requires output selection.
  -i, --index <INDEX>     Workspaces are ordered by wayland protocol id. Global or on selected output.
  -n, --name <NAME>       Global or on selected output.
  -p, --protocol-id <ID>  Wayland protocol id used in communication between server and client.

Output selection (exclusive):
  -o, --output-name <OUTPUT_NAME>
  -u, --output-protocol-id <OUTPUT_ID>
```

```
$ wsctrl deactivate --index 3 --output-name eDP-1
Error: "Unable to find workspace with index 3"
```

```
$ wsctrl ls
# Name Description Location GlobalId ProtocolId GroupId
0 eDP-1 "11_24_6 - 23085 - eDP-1" (2560, 0) 36 3 4278190080
    # Name States Coords ProtocolId
    0 None {Active} None 4278190081
1 DP-4 "Unknown - Unknown - DP-4" (0, 0) 41 4 4278190082
    0 None {Active} None 4278190083
    1 None {} None 4278190084
    2 gaming {} None 4278190085
    3 None {} None 4278190086
    4 mail {} None 4278190087
    5 terminal {} None 4278190088
```

```
$ wsctrl ls --output-name eDP-1 --json | jq
[
  {
    "output": {
      "protocolId": 3,
      "name": "eDP-1",
      "location": [
        2560,
        0
      ],
      "description": "11_24_6 - 23085 - eDP-1",
      "globalId": 36
    },
    "group_handle": 4278190080,
    "workspaces": [
      {
        "handle": 4278190081,
        "name": null,
        "coordinates": null,
        "state": [
          "Active"
        ]
      },
      {
        "handle": 4278190082,
        "name": null,
        "coordinates": null,
        "state": []
      }
    ]
  }
]
```



# TODO

* fix or remove -outputs-only
* do not require output slection if only one output is connected
* option to select workspace by urgent/hidden/coords
* sanitize name input when creating new workspace (length, symbols)
* select output by location/index
* implement moving of workspaces between groups(outputs)
* handle multiple active workspaces in same group on --active
* add next/prev (+1/-1) for index selection
* arg to deactivate prev/all ws on activate
* implement list only hidden/urgent/active
* order workspaces by coords
* tests
* check caps before request
* make group/output optional to unassign workspace(?)
* show caps / cli arg to request caps
* cli arg to set tiling state

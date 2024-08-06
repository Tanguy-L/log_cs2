# CS2 Logs
A small console project to display elements based on filters. And with colors !
## Getting Started

be careful to the path of the file cs2server.log.

Read logs from log file -> ../cs2server.log

```
cargo run
```

## Filters

A filter looks like this in filters.json

```JSON

{
    "name": "TV",
    "status": "Infos",
    "key_code": "a",
    "regex": ".*(?:TV).*",
    "rule": "OneLine",
    "is_on": true
},
```

- Status can be Infos, Warning, Error, Custom, Custom2, Custom3, each one define the color of the line
- KeyCode for keybind to toggle the filter
- Rule is OneLine or Verbose (if you take x lines after the regex)

## Roadmap

- [ ] Define the path of file log on launch
- [ ] Refactor this bad boy
- [ ] Use the OneLine rule
- [ ] Use .env WTF !

That pretty much it for now !
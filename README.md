# kairowiki [![Test](https://github.com/foldu/kairowiki/workflows/Test/badge.svg)](https://github.com/foldu/kairowiki/actions) [![](https://img.shields.io/docker/v/foldu/kairowiki)](https://hub.docker.com/r/foldu/kairowiki)
## Development
```shell
source .env
test -f "$DATABASE_FILE" || ./init_db.sh
cargo r
```

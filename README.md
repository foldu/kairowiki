# kairowiki [![Test](https://github.com/foldu/kairowiki/workflows/Test/badge.svg)](https://github.com/foldu/kairowiki/actions) [![](https://img.shields.io/docker/v/foldu/kairowiki)](https://hub.docker.com/r/foldu/kairowiki)
## Development
```shell
source .env
(cd web && yarn install --frozen-lockfile && yarn run webpack)
# watch for file changes and rebuild on change
# cd web && yarn run webpack -w
test -f "$DATABASE_FILE" || ./init_db.sh
cargo r
```

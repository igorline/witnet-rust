# Custom TOML configuration file

A custom `witnet.toml` file can be used to configure parameters of the node. In order for the node to be able to read this file, it should exist in the current working directory where the node is run. Another way is to just tell the node where the config file resides using a command line option. See the CLI reference for more info.

## TOML file example

``` toml
[connections] # section for connections-related params
server_addr = "127.0.0.1:1234"
inbound_limit = 128
outbound_limit = 1
known_peers = ["52.166.178.145:21337"]
bootstrap_peers_period_seconds = 30
storage_peers_period_seconds = 30
handshake_timeout_seconds = 5

[storage] # section for storage-related params
db_path = ".wit"

[consensus_constants] # consensus-critical constants
checkpoint_zero_timestamp = 1548855420
checkpoints_period_seconds = 90

[jsonrpc] # section for params related to JSON-RPC API
enabled = true
server_address = "127.0.0.1:4321"

[mining] # mining-related params
enabled = true

# ... more options
```

## Configuration params

| Section               | Param                            | Default Value              | Description                                                         |
|-----------------------|----------------------------------|----------------------------|---------------------------------------------------------------------|
| `connections`         | `server_addr`                    | `"127.0.0.1:21337"`        | Server socket address to which it should bind to                    |
| `connections`         | `inbound_limit`                  | `128`                      | Maximum number of concurrent connections the server should accept   |
| `connections`         | `outbound_limit`                 | `8`                        | Maximum number of opened connections to other peers this node has   |
| `connections`         | `known_peers`                    | `["52.166.178.145:21337"]` | Other peer addresses this node knows about at start                 |
| `connections`         | `bootstrap_peers_period_seconds` | `5`                        | Period of the outbound peer bootstrapping process (in seconds)      |
| `connections`         | `discovery_peers_period_seconds` | `5`                        | Period of the outbound peer discovery process (in seconds)          |
| `connections`         | `handshake_timeout_seconds`      | `5`                        | Timeout for the handshake process (in seconds)                      |
| `connections`         | `blocks_timeout_secconds`        | `400`                      | Number of seconds before giving up waiting for requested blocks     |
| `storage`             | `db_path`                        | `".witnet-rust-testnet-3"` | Directory containing the database files                             |
| `storage`             | `peers_period_seconds`           | `30`                       | Period of the known peers backup into storage process (in seconds)  |
| `consensus_constants` | `activity_period`                | `40`                       | Number of recent epochs to comput for witness activity metric       |
| `consensus_constants` | `checkpoint_zero_timestamp`      | `1559347200`               | Timestamp at checkpoint 0 (the start of epoch 0)                    |
| `consensus_constants` | `checkpoints_period_seconds`     | `90`                       | Seconds between the start of an epoch and the start of the next one |
| `consensus_constants` | `max_block_weight`               | `10000`                    | Maximum size for each block in the chain                            |
| `consensus_constants` | `reputation_issuance`            | `1`                        | How many reputation points to issue per each witnessing act         |
| `consensus_constants` | `reputation_issuance_stop`       | `1048576`                  | Number of witnessing acts before reputation issuance halts          |
| `consensus_constants` | `reputation_expire_alpha_diff`   | `20000`                    | Number of witnessing acts before a reputation point expires         |
| `consensus_constants` | `reputation_penalization_factor  | `0.5`                      | Fraction of reputation lost by witnesses being out of consensus     |
| `jsonrpc`             | `enabled`                        | `true`                     | Enable JSON-RPC server                                              |
| `jsonrpc`             | `server_address`                 | `"127.0.0.1:21338"`        | JSON-RPC server socket address                                      |
| `mining`              | `enabled`                        | `true`                     | Enable MiningManager                                                |

These are the defaults for `testnet-3`.
See [environment][environment] for the specific values for all the environments.

The parameters in the `[consensus_constants]` section are ignored when the
environment is set to `mainnet`.

[environment]: environment.md

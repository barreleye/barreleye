# Barreleye

[![Github Actions](https://img.shields.io/github/actions/workflow/status/barreleye/barreleye/tests.yml?style=flat-square)](https://github.com/barreleye/barreleye/actions)
[![Dependency Status](https://deps.rs/repo/github/barreleye/barreleye/status.svg?style=flat-square)](https://deps.rs/repo/github/barreleye/barreleye)
[![License](https://img.shields.io/github/license/barreleye/barreleye?color=orange&style=flat-square)](/LICENSE)
[![Downloads](https://img.shields.io/crates/d/barreleye?color=blue&style=flat-square)](https://crates.io/crates/barreleye)
![Activity](https://img.shields.io/github/commit-activity/m/barreleye/barreleye?style=flat-square)
[![Discord](https://img.shields.io/discord/1026664296861679646?style=flat-square&color=blue)](https://discord.gg/VX8PdWSwNZ)

⚠️ This is an actively developed work-in-progress and not yet ready for production. Use at your own risk ⚠️

## What is Barreleye?

Barreleye is an open-source **entity-based blockchain analytics** tool. It can show what digital assets an address has and trace back where they came from.

Features:

1. **Simple.** Easy to get started with on a single machine.
1. **Scalable.** Optimized for demanding business use-cases.
1. **Extendable.** API-based interface so it can be integrated into other systems.
1. **Multi-chain.** Supports Bitcoin and EVM-based networks (with ability to add more).

## Download

<!-- ### Via package manager (not recommended right now; outdated)

```bash
cargo install barreleye
```

### From source -->

Requires Rust 1.65.0+:

```bash
git clone https://github.com/barreleye/barreleye
cd barreleye
cargo build && cargo install
```

## Try

To run Barreleye locally:

```bash
./barreleye
```

This will do the following:

- Run migrations (including seeding with a random public Ethereum RPC node)
- Start the server, which will handle admin & analytics API requests
- Start the indexer, which will:
  - Store extracted blockchain data in Parquet files locally
  - Store relational data in SQLite locally
  - Store warehouse data in DuckDB locally

For production you'd probably want to store extracted blockchain data in anything S3-compatible (eg: R2, GCS, MinIO, etc):

```bash
./barreleye \
  --storage http://s3.us-east-1.amazonaws.com/bucket_name/
# --storage http://storage.googleapis.com/bucket_name/
```

You can also use a hosted RDBMS like PostgreSQL or MySQL instead of SQLite:

```bash
./barreleye \
  --storage http://s3.us-east-1.amazonaws.com/bucket/ \
  --database postgres://username:password@postgres-host:5432/database_name
# --database mysql://username:password@mysql-host:3306/database_name
```

And a hosted warehouse OLAP instead of DuckDB (currently only Clickhouse is supported):

```bash
./barreleye \
  --storage http://s3.us-east-1.amazonaws.com/bucket/ \
  --database postgres://username:password@localhost:5432/database_name \
  --warehouse http://username:password@localhost:8123/database_name
```

Finally, to speed up indexing run your own Ethereum archive node with a higher rate-limit (rps = requests per second):

```bash
curl -X PUT \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <API_KEY>" \
  -d '{
    "rpcNode": "<YOUR_OWN_RPC_NODE_URL>",
    "rps": 1500
  }' \
  http://localhost:4000/v0/networks/net_ethereum
```

## Add other networks

For now, Barreleye works with Bitcoin and EVM-compatible chains.

A default API key is generated on the first run, so to get it - connect to your RDBMS and run:

```sql
select uuid from api_keys;
```

Add a Bitcoin RPC node:

```bash
curl -X POST \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <API_KEY>" \
  -d '{
    "name": "Bitcoin",
    "env": "mainnet",
    "blockchain": "bitcoin",
    "chainId": 0,
    "blockTimeMs": 600000,
    "rpcEndpoints": ["http://username:password@127.0.0.1:8332"],
    "rps": 100
  }' \
  http://localhost:4000/v0/networks
```

Add an EVM-based RPC node:

```bash
curl -X POST \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <API_KEY>" \
  -d '{
    "name": "Ethereum",
    "env": "mainnet",
    "blockchain": "evm",
    "chainId": 1,
    "blockTimeMs": 12000,
    "rpcEndpoints": ["http://127.0.0.1:8545"],
    "rps": 100
  }' \
  http://localhost:4000/v0/networks
```

⏳ Indexing will take a while. To monitor progress:

```bash
curl -X GET \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <API_KEY>" \
  http://localhost:4000/v0/stats
```

## Analytics

To get networks, assets, labels, etc:

```bash
curl -X GET \
  -H "Content-Type: application/json" \
  http://localhost:4000/v0/info?address=<BLOCKCHAIN_ADDRESS>
```

To find connected labeled addresses that might have funded the requested address through multiple hops:

```bash
curl -X GET \
  -H "Content-Type: application/json" \
  http://localhost:4000/v0/upstream?address=<BLOCKCHAIN_ADDRESS>
```

## Random Notes

- Be aware of your RPC node limits. Indexer makes a significant amount of RPC calls to index historical and new blocks.
- For indexing, you might have to set Clickhouse's `max_server_memory_usage_to_ram_ratio` to `2` ([read more](https://github.com/ClickHouse/ClickHouse/issues/17631))

## Get Involved

To stay in touch with Barreleye:

- Star this repo ★
- Follow on [Twitter](https://twitter.com/BarreleyeLabs)
- Join on [Discord](https://discord.gg/VX8PdWSwNZ)
- [Contribute](/CONTRIBUTING.md) -- pull requests are welcome (for major changes, please open an issue first to discuss what you would like to change)

## License

Source code for Barreleye is variously licensed under a number of different licenses. A copy of each license can be found in [each repository](https://github.com/barreleye).

- Libraries and SDKs, each located in its own distinct repository, are released under either the [Apache License 2.0](https://opensource.org/licenses/Apache-2.0) or [MIT License](https://opensource.org/licenses/MIT).
- Core code for Barreleye, located in [this repository](https://github.com/barreleye/barreleye), is released under the [GNU Affero General Public License 3.0](/LICENSE).

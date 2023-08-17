# Barreleye

[![Github Actions](https://img.shields.io/github/actions/workflow/status/barreleye/barreleye/tests.yml?style=flat-square)](https://github.com/barreleye/barreleye/actions)
[![Dependency Status](https://deps.rs/repo/github/barreleye/barreleye/status.svg?style=flat-square)](https://deps.rs/repo/github/barreleye/barreleye)
[![License](https://img.shields.io/github/license/barreleye/barreleye?color=orange&style=flat-square)](/LICENSE)
[![Discord](https://img.shields.io/discord/1026664296861679646?style=flat-square&color=blue)](https://discord.gg/VX8PdWSwNZ)

> **Note**
> ⚠️ This is an actively developed work-in-progress and not yet ready for production. Use at your own risk

## What is Barreleye?

Barreleye is an open-source **blockchain KYC** tool. It can trace the flow and amount of funds to & from risky addresses.

Features:

1. **Simple.** Easy to get started with on a single machine.
1. **Scalable.** Optimized for demanding business use-cases.
1. **Extendable.** API-based interface so it can be integrated into other systems.
1. **Multi-chain.** Supports Bitcoin and EVM-based networks (with ability to add more).

## Get Started

Clone, build & install:

```bash
git clone https://github.com/barreleye/barreleye
cd barreleye
cargo build
cargo install
```

> **Note**
> ⚠️ [ClickHouse](https://github.com/ClickHouse/ClickHouse) is a requirement for Barreleye

Run locally (pointing to your [ClickHouse](https://github.com/ClickHouse/ClickHouse) instance):

```bash
barreleye \
  --warehouse http://username:password@localhost:8123/database_name
```

This will do the following:

- Start the server, which will handle future API requests
- Start the indexer, which will:
  - Store extracted blockchain data in Parquet files locally
  - Store relational data in [SQLite](https://www.sqlite.org/) locally
  - Store warehouse data in [DuckDB](https://duckdb.org/) locally

For production you'll probably want to store extracted blockchain data in the cloud (eg: Amazon S3, Cloudflare R2, etc), as opposed to your local files:

```bash
barreleye \
  --warehouse http://username:password@localhost:8123/database_name \
  --storage http://s3.us-east-1.amazonaws.com/bucket_name/
```

You can also use a hosted RDBMS like PostgreSQL or MySQL instead of SQLite:

```bash
barreleye \
  --warehouse http://username:password@localhost:8123/database_name \
  --storage http://s3.us-east-1.amazonaws.com/bucket_name/ \
  --database postgres://username:password@postgres-host:5432/database_name
```

## Add Custom Networks

You have to add network nodes in order for indexer to start processing data.

A default API key is generated on the first run, so to get it - connect to your RDBMS and run:

```sql
select uuid from api_keys;
```

Add a Bitcoin RPC node (`-txindex` is required):

```bash
curl -X POST \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $YOUR_API_KEY" \
  -d '{
    "name": "Bitcoin",
    "architecture": "bitcoin",
    "blockTime": 600000,
    "rpcEndpoint": "http://username:password@127.0.0.1:8332"
  }' \
  http://localhost:4000/v1/networks
```

Add an EVM-based RPC node (archive node is required):

```bash
curl -X POST \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $YOUR_API_KEY" \
  -d '{
    "name": "Ethereum",
    "architecture": "evm",
    "blockTime": 12000,
    "rpcEndpoint": "http://127.0.0.1:8545"
  }' \
  http://localhost:4000/v1/networks
```

⏳ Indexing will take a while. To monitor progress:

```bash
curl -X GET \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $YOUR_API_KEY" \
  http://localhost:4000/v1/stats
```

## Analytics

To get networks, assets, labels, etc:

```bash
curl -X GET \
  -H 'Content-Type: application/json' \
  http://localhost:4000/v1/info?address=$BLOCKCHAIN_ADDRESS
```

To find connected labeled addresses that might have funded the requested address through multiple hops:

```bash
curl -X GET \
  -H 'Content-Type: application/json' \
  http://localhost:4000/v1/upstream?address=$BLOCKCHAIN_ADDRESS
```

## Random Notes

- Be aware of your RPC node limits. Indexer makes a significant amount of RPC calls to index historical and new blocks.
- For indexing, you might have to set ClickHouse's `max_server_memory_usage_to_ram_ratio` to `2` ([read more](https://github.com/ClickHouse/ClickHouse/issues/17631))

## Get Involved

To stay in touch with Barreleye:

- Star this repo ★
- Follow on [Twitter](https://twitter.com/BarreleyeLabs)
- Join on [Discord](https://discord.gg/VX8PdWSwNZ)
- [Contribute](/CONTRIBUTING.md) -- pull requests are welcome (for major changes, please open an issue first to discuss what you would like to change)

## License

Source code for Barreleye is variously licensed under a number of different licenses. A copy of each license can be found in [each repository](https://github.com/barreleye).

- Core code for Barreleye, located in [this repository](https://github.com/barreleye/barreleye), is released under the [GNU Affero General Public License 3.0](/LICENSE).
- Libraries and SDKs, each located in its own distinct repository, are released under either the [Apache License 2.0](https://opensource.org/licenses/Apache-2.0) or [MIT License](https://opensource.org/licenses/MIT).

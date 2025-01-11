# Barreleye

[![Github Actions](https://img.shields.io/github/actions/workflow/status/barreleye/barreleye/tests.yml)](https://github.com/barreleye/barreleye/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/barreleye/barreleye?tab=readme-ov-file#%EF%B8%8F-license)

**Barreleye** is an open-source multi-chain blockchain indexer and explorer.

> [!IMPORTANT]
> This is a work-in-progress and not yet ready for use or production.

Features:

- **Easy to use** — start on a single machine, scale up as needed
- **Scalable** — optimized for handling lots of data
- **Multi-chain** — designed to support multiple blockchain architectures

## Requirements

1. Rust v1.83+ — if you're compiling from source
1. Blockchain nodes — for indexing (eg: [Bitcoin](https://bitcoin.org/), [Ethereum](https://ethereum.org/), etc)

## Get Started

Clone, build & install:

```sh
git clone https://github.com/barreleye/barreleye
cd barreleye
cargo build
cargo install
```

Run locally (pointing to your [ClickHouse](https://github.com/ClickHouse/ClickHouse) instance):

```sh
cargo run
```

This will do the following:

- Start the server, which will handle API requests
- Start the indexer, which will:
  - Store extracted blockchain data in Parquet files
  - Store relational data in [SQLite](https://www.sqlite.org/)
  - Store warehouse data in [DuckDB](https://duckdb.org/)

For production you'll probably want to store extracted blockchain data in the cloud (eg: [Amazon S3](https://aws.amazon.com/s3/), [Cloudflare R2](https://www.cloudflare.com/developer-platform/r2/), etc), as opposed to your localhost:

```sh
cargo run -- \
  --warehouse http://example.clickhouse.cloud:8123/database_name \
  --storage http://s3.us-east-1.amazonaws.com/bucket_name/
```

You can also use a hosted RDBMS like [PostgreSQL](https://www.postgresql.org/) or [MySQL](https://www.mysql.com/) instead of SQLite:

```sh
cargo run -- \
  --warehouse http://example.clickhouse.cloud:8123/database_name \
  --storage http://s3.us-east-1.amazonaws.com/bucket_name/ \
  --database postgres://postgres-host:5432/database_name
```

## Modes

Barreleye operates two parallel components: the indexer and the server. The indexer retrieves blockchain data, while the server manages API requests, handling data and address information.

Barreleye automatically runs both the indexer and server in parallel by default:

```sh
cargo run
```

To run only the indexer:

```sh
cargo run -- --mode indexer
```

To run only the server:

```sh
cargo run -- --mode http
```

> [!TIP]
> The indexer operates in failover mode, with a single primary instance running while secondary instances stand by, ready to take over if the primary fails.

## Data Management

Barreleye does not come with any pre-defined data. Instead, it gives you the ability to add and manage data yourself. The API calls below give an overview of how to manage data.

**Add Blockchains**

Add a Bitcoin RPC node:

```sh
curl -X POST \
  -H 'Content-Type: application/json' \
  -d '{
    "id": "net_bitcoin",
    "name": "Bitcoin",
    "architecture": "bitcoin",
    "blockTime": 600000,
    "rpcEndpoint": "http://username:password@127.0.0.1:8332"
  }' \
  http://localhost:4000/v1/networks
```

Add an EVM-based RPC node (archive node is required):

```sh
curl -X POST \
  -H 'Content-Type: application/json' \
  -d '{
    "id": "net_ethereum",
    "name": "Ethereum",
    "architecture": "evm",
    "chainId": 1,
    "blockTime": 12000,
    "rpcEndpoint": "http://127.0.0.1:8545"
  }' \
  http://localhost:4000/v1/networks
```

**Add Tokens**

To add native Bitcoin currency:

```sh
curl -X POST \
  -H 'Content-Type: application/json' \
  -d '{
    "network": "net_bitcoin",
    "name": "bitcoin",
    "symbol": "BTC",
    "decimals": 8
  }' \
  http://localhost:4000/v1/tokens
```

To add native Ethereum currency:

```sh
curl -X POST \
  -H 'Content-Type: application/json' \
  -d '{
    "network": "net_ethereum",
    "name": "Ether",
    "symbol": "ETH",
    "decimals": 18
  }' \
  http://localhost:4000/v1/tokens
```

To add an ERC-20 token:

```sh
curl -X POST \
  -H 'Content-Type: application/json' \
  -d '{
    "network": "net_ethereum",
    "name": "USD Coin",
    "symbol": "USDC",
    "address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
    "decimals": 6
  }' \
  http://localhost:4000/v1/tokens
```

**Add Tags**

```sh
curl -X POST \
  -H 'Content-Type: application/json' \
  -d '{
    "id": "tag_exchange",
    "name": "Exchange",
    "riskLevel": "low"
  }' \
  http://localhost:4000/v1/tags
```

**Add Entities**

An entity can be an item that contains one or several blockchain addresses:

```sh
curl -X POST \
  -H 'Content-Type: application/json' \
  -d '{
    "id": "ent_coinbase",
    "name": "Coinbase",
    "description": "",
    "tags": ["tag_exchange"]
  }' \
  http://localhost:4000/v1/entities
```

To add addresses:

```sh
curl -X POST \
  -H 'Content-Type: application/json' \
  -d '{
    "entity": "ent_coinbase",
    "network": "net_ethereum",
    "addresses": [
      {
        "address": "0x71660c4005BA85c37ccec55d0C4493E66Fe775d3",
        "description": "Address #1"
      }, {
        "address": "0x503828976d22510aad0201ac7ec88293211d23da",
        "description": "Address #2"
      }
    ]
  }' \
  http://localhost:4000/v1/addresses
```

## Address Info

To query information about a particular blockchain address:

```sh
curl -X GET \
  -H 'Content-Type: application/json' \
  http://localhost:4000/v1/info?q=<BLOCKCHAIN_ADDRESS>
```

## Random Notes

- Be aware of your RPC node limits. Indexer makes a significant amount of RPC calls to index historical and new blocks.
- For indexing, you might have to set ClickHouse's `max_server_memory_usage_to_ram_ratio` to `2` ([read more](https://github.com/ClickHouse/ClickHouse/issues/17631))

## License

Barreleye is free, open source and permissively licensed.
Except where noted (below and/or in individual files), all code in this repository is dual-licensed under either:

* MIT License ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option.
This means you can select the license you prefer.

**Your contributions**

Unless you explicitly state otherwise,
any contribution intentionally submitted for inclusion in the work by you,
as defined in the Apache-2.0 license,
shall be dual licensed as above,
without any additional terms or conditions.
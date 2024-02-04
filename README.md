# [Barreleye](https://barreleye.com/)

[![Github Actions](https://img.shields.io/github/actions/workflow/status/barreleye/barreleye/tests.yml)](https://github.com/barreleye/barreleye/actions)
[![License](https://img.shields.io/github/license/barreleye/barreleye)](/LICENSE)
[![Discord](https://img.shields.io/discord/1026664296861679646?logo=discord&logoColor=ffffff&label=discord)](https://discord.gg/VX8PdWSwNZ)
[![X (formerly Twitter) Follow](https://img.shields.io/twitter/follow/GetBarreleye)](https://twitter.com/GetBarreleye)

## What is Barreleye?

Barreleye is an open-source blockchain analytics tool. It's entity based, so it can answer questions like what assets an entity has and where they came from.

> **Note**
> This is an actively developed work-in-progress and not yet ready for production. Use at your own risk.

## ✨ Features

- ⛵️ **Easy of use** — start on a single machine, scale up as needed
- 🚢 **Scalable** — optimized for demanding business use-cases
- 🥳 **Self-hosted** — API-based interface that can be integrated into other systems
- 💪 **Multi-chain** — designed to support multiple blockchain architectures

## 🧰 Requirements

1. [Rust](https://www.rust-lang.org/) v1.70+ — if you're compiling from source
1. [ClickHouse](https://github.com/ClickHouse/ClickHouse) v23.5+ — for warehouse data storage
1. Blockchain nodes — for indexing (eg: [Bitcoin](https://bitcoin.org/), [Ethereum](https://ethereum.org/), etc)

> **Note**
> ⚠️ EVM-based chains are not yet supported (this is a work-in-progress)

## 👩‍💻 Get Started

Clone, build & install:

```bash
git clone https://github.com/barreleye/barreleye
cd barreleye
cargo build
cargo install
```

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

For production you'll probably want to store extracted blockchain data in the cloud (eg: [Amazon S3](https://aws.amazon.com/s3/), [Cloudflare R2](https://www.cloudflare.com/developer-platform/r2/), etc), as opposed to your local files:

```bash
barreleye \
  --warehouse http://username:password@localhost:8123/database_name \
  --storage http://s3.us-east-1.amazonaws.com/bucket_name/
```

You can also use a hosted RDBMS like [PostgreSQL](https://www.postgresql.org/) or [MySQL](https://www.mysql.com/) instead of SQLite:

```bash
barreleye \
  --warehouse http://username:password@localhost:8123/database_name \
  --storage http://s3.us-east-1.amazonaws.com/bucket_name/ \
  --database postgres://username:password@postgres-host:5432/database_name
```

## 📦 Modes

Barreleye is bundled with the indexer and the server in the same program. The indexer is responsible for crawling blockchains and retrieving all the necessary data, while the server is focused on handling API requests (data management + analytics requests).

By default, both the indexer and the server are enabled and will run in parallel:

```bash
barreleye
```

To run only the indexer:

```bash
barreleye --mode indexer
```

To run only the server:

```bash
barreleye --mode http
```

> **Note**
> ⚠️ Indexer is designed to run in failover mode. Only the primary instance will run at once; the others will wait for the primary to fail in order to promote a secondary.

## 💾 Add Data

Barreleye does not come with any pre-defined data. Instead, it gives you the ability to add and manage data yourself. The API calls below give an overview of how to manage data.

A default API key is generated when you first start Barreleye, so to get it — connect to your RDBMS and retrieve the only key that has been auto-created:

```sql
select uuid from api_keys; -- will be $YOUR_API_KEY in examples below
```

### Add Blockchains

Add a Bitcoin RPC node:

```bash
curl -X POST \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $YOUR_API_KEY" \
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

```bash
curl -X POST \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $YOUR_API_KEY" \
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

### Add Tokens

To add native Bitcoin currency:

```bash
curl -X POST \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $YOUR_API_KEY" \
  -d '{
    "network": "net_bitcoin",
    "name": "bitcoin",
    "symbol": "BTC",
    "decimals": 8
  }' \
  http://localhost:4000/v1/tokens
```

To add native Ethereum currency:

```bash
curl -X POST \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $YOUR_API_KEY" \
  -d '{
    "network": "net_ethereum",
    "name": "Ether",
    "symbol": "ETH",
    "decimals": 18
  }' \
  http://localhost:4000/v1/tokens
```

To add an ERC-20 token:

```bash
curl -X POST \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $YOUR_API_KEY" \
  -d '{
    "network": "net_ethereum",
    "name": "USD Coin",
    "symbol": "USDC",
    "address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
    "decimals": 6
  }' \
  http://localhost:4000/v1/tokens
```

### Add Tags

```bash
curl -X POST \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $YOUR_API_KEY" \
  -d '{
    "id": "tag_exchange",
    "name": "Exchange",
    "riskLevel": "low"
  }' \
  http://localhost:4000/v1/tags
```

### Add Entities

An entity can be an item that contains one or several blockchain addresses:

```bash
curl -X POST \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $YOUR_API_KEY" \
  -d '{
    "id": "ent_coinbase",
    "name": "Coinbase",
    "description": "",
    "tags": ["tag_exchange"]
  }' \
  http://localhost:4000/v1/entities
```

To add addresses:

```bash
curl -X POST \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $YOUR_API_KEY" \
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

## 📊 Analytics

To query information about a particular blockchain address:

```bash
curl -X GET \
  -H 'Content-Type: application/json' \
  http://localhost:4000/v1/info?q=$BLOCKCHAIN_ADDRESS
```

## 🗒 Random Notes

- Be aware of your RPC node limits. Indexer makes a significant amount of RPC calls to index historical and new blocks.
- For indexing, you might have to set ClickHouse's `max_server_memory_usage_to_ram_ratio` to `2` ([read more](https://github.com/ClickHouse/ClickHouse/issues/17631))

## 🥹 Get Involved

To stay in touch with Barreleye:

- Star this repo ★
- Follow on [Twitter](https://twitter.com/GetBarreleye)
- Join on [Discord](https://discord.gg/VX8PdWSwNZ)
- [Contribute](/CONTRIBUTING.md) -- pull requests are welcome (for major changes, please open an issue first to discuss what you would like to change)

## ⚖️ License

Source code for Barreleye is variously licensed under a number of different licenses. A copy of each license can be found in [each repository](https://github.com/barreleye).

- Core code for Barreleye, located in [this repository](https://github.com/barreleye/barreleye), is released under the [Apache 2.0](/LICENSE).
- Libraries and SDKs, each located in its own distinct repository, are released under either the [Apache License 2.0](https://opensource.org/licenses/Apache-2.0) or [MIT License](https://opensource.org/licenses/MIT).

# Barreleye

[![](https://img.shields.io/badge/status-beta-ff69b4.svg?style=flat-square)](https://github.com/barreleye/barreleye)
[![Contributions](https://img.shields.io/badge/contributions-welcome-ff69b4?style=flat-square)](/CONTRIBUTING.md "Go to contributions doc")
[![Github Actions](https://img.shields.io/github/actions/workflow/status/barreleye/barreleye/tests.yml?style=flat-square)](https://github.com/barreleye/barreleye/actions)
[![Dependency Status](https://deps.rs/repo/github/barreleye/barreleye/status.svg?style=flat-square)](https://deps.rs/repo/github/barreleye/barreleye)
[![Crates.io](https://img.shields.io/crates/v/barreleye?color=brightgreen&style=flat-square)](https://crates.io/crates/barreleye)
[![License](https://img.shields.io/github/license/barreleye/barreleye?color=9cf&style=flat-square)](/LICENSE)
[![Downloads](https://img.shields.io/crates/d/barreleye?color=blue&style=flat-square)](https://crates.io/crates/barreleye)
![Activity](https://img.shields.io/github/commit-activity/m/barreleye/barreleye?style=flat-square)
[![Discord](https://img.shields.io/discord/1026664296861679646?style=flat-square&color=blue)](https://discord.gg/VX8PdWSwNZ)
[![Twitter](https://img.shields.io/twitter/follow/barreleyelabs?color=blue&style=flat-square)](https://twitter.com/BarreleyeLabs)

## What is Barreleye?

Barreleye is an open-source, multi-chain blockchain analytics tool. It's goal is to help answer the following questions:

1. What assets does an address hold?
1. Where did these assets come from?
1. What other wallets might be related?

**Note:** This is an actively developed work-in-progress and not yet ready for production. Use at your own risk

## Try

### Via package manager

```bash
cargo install barreleye
barreleye --help
```

### From source

Requires Rust 1.65.0+:

```bash
git clone https://github.com/barreleye/barreleye
cd barreleye
cargo run -- --help
```

Notes:

- A default config file will be generated on the first run. Optionally, rename `barreleye.sample.toml` to `barreleye.toml`

- [Clickhouse](https://github.com/ClickHouse/ClickHouse) is a requirement for warehouse data storage (default configs point to a locally running server)

- Out of the box Barreleye is configured to use [SQLite](https://www.sqlite.org/) ([MySQL](https://www.mysql.com/) and [PostgreSQL](https://www.postgresql.org/) are also supported)

## Basics

Barreleye consists of two parts: the indexer and the server. The indexer connects to specified RPC nodes to process blocks, and the server handles management and analytics requests.

**Note:** Indexing not only keeps up with continuous block creation, but also runs through historical blocks. This process makes significant amount of RPC calls.

To start just the indexer, without the server (only one indexer will be active at a time): `cargo run -- --indexer`

To run the server without indexing: `cargo run -- --server`

To run them all together: `cargo run`

## Add networks

A default API key is generated on the first run, so to get it:

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
    "tag": "Bitcoin",
    "env": "mainnet",
    "blockchain": "bitcoin",
    "chainId": 0,
    "blockTimeMs": 600000,
    "rpcEndpoints": ["http://username:password@127.0.0.1:8332"],
    "rps": 100
  }' \
  http://localhost:22775/v0/networks
```

Add an EVM-based RPC node:

```bash
curl -X POST \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <API_KEY>" \
  -d '{
    "name": "Ethereum",
    "tag": "Ethereum",
    "env": "mainnet",
    "blockchain": "evm",
    "chainId": 1,
    "blockTimeMs": 12000,
    "rpcEndpoints": ["http://127.0.0.1:8545"],
    "rps": 100
  }' \
  http://localhost:22775/v0/networks
```

⏳ Indexing will take a while.

## Analytics

To get networks, assets, labels, etc:

```bash
curl -X GET \
  -H "Content-Type: application/json" \
  http://localhost:22775/v0/info?address=<BLOCKCHAIN_ADDRESS>
```

To find connected labeled addresses that might have funded through multiple hops:

```bash
curl -X GET \
  -H "Content-Type: application/json" \
  http://localhost:22775/v0/upstream?address=<BLOCKCHAIN_ADDRESS>
```

## Random Notes

- Be aware of your RPC node limits. Indexer makes a significant amount of RPC calls to index historical and new blocks.
- For indexing, you might have to set Clickhouse's `max_server_memory_usage_to_ram_ratio` to `2` ([read more](https://github.com/ClickHouse/ClickHouse/issues/17631))
- Warehouse's `experimental_relations` table (along with modules) is not accurate and should not be relied on right now

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

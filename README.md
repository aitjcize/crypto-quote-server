Crypto Quote Server
===================

A simple crypto price quote server to be used together with Google spreadsheet.
Price data taken from Coingecko.

Usage
-----

Host the server somewhere (e.g. GCP, AWS).

Get Coin/Token Price:
```
=IMPORTXML("https://YOUR_SERVER_HOST/quote?id=bitcion", "//Quote/price/text()")
```

Get Wallet Balance:
```
=IMPORTXML("https://YOUR_SERVER_HOST/wallet_balance?chain_id=<CHAIN_ID>&address=<YOUR_ADDRESS>", "//WalletBalance/balance/text()")
```

Supported Chain ID:
* ethereum
* polygon
* avalanche
* polkadot
* bsc

Get Wallet ERC20 Token Balance:
```
=IMPORTXML("https://YOUR_SERVER_HOST/wallet_balance?chain_id=<CHAIN_ID>&token_address=<ERC20_CONTRACT_ADDRESS>&address=<YOUR_ADDRESS>", "//WalletBalance/balance/text()")
```

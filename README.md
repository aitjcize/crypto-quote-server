Crypto Quote Server
===================

A simple crypto price quote server to be used together with Google spreadsheet.
Price data taken from Coingecko.

Usage
-----

Host the server somewhere (e.g. GCP, AWS).

In google spreadsheet, you can use the following formulat to fetch the price
info:

```
=IMPORTXML("https://YOUR_SERVER_HOST/quote?name=bitcion", "//Quote/price")
```

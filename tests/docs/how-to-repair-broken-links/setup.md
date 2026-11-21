```console
$ zettel init
initialized a zettel store at [CWD]

$ zettel note create "Retry with backoff" -t retry -p human -b 'Retry a 503 response with exponential backoff.'
ab12-retry-with-backoff

$ zettel note create "Use the shared HTTP client" -t http -p human -l retry-policy -b 'The shared client applies the retry rules.'
cd34-use-the-shared-http-client

```


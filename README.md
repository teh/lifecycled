# lifecycled - a local file sytem lifecycle daemon

This project runs a simple daemon that periodically checks glob/date
patterns against the local filesystem and runs commands after files
have reached a certain age (based on `strptime`-like matching for now).

E.g. the following rule check for rotated ngix logs older than 2 days.
Matching files are compressed and shipped to s3.

```toml
[rules.compress_ship]
match = "/var/log/nginx/rotated.%Y-%m-%d.log"
after = "2d"
run = ["zstd --rm ${LIFECYCLED_PATH}", "aws s3 cp ${LIFECYCLED_PATH} s3://backup-bucket/$(basename ${LIFECYCLED_PATH})"]
```

Commands in the `run` list are executed via bash, with the `LIFECYCLED_PATH` environment variable pointing to the matching path. See `./examples` for the config examples.

`match` can use `*` for a simple glob, but not `**` or `[]`, `{a,b}` or anything fancy like that yet.

# Buyer Beware!

* Running glob expressions unsupervised is dangerous - only use lifecycled if you know what you are doing
* lifecycled needs access to the files its matching, this usually means running as root which is dangerous as well
* lifecycled expects you to rename files in a way so they no longer match the rule. Otherwise they will re-execute every 60 seconds.


# Roadmap

* `--dry-run` mode which fast-forwards through time to simulate what would happen
* forking with new effective user ID per rule to avoid runnig as root
* ctime/mtime support
* `duration` as a matching parameter so we can run e.g. `ctime + 7 days`
* more fancy glob matching
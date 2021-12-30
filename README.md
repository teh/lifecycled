# in scope

* dealing with files after they have been written and are done in some sense
* arbitrary commands like compression
* ctime or regex + strptime to detect age
* dry-run with --asof time to simulate what would happen
* allow sth like "?ctime+7d" relative to ctime?

# out of scope

* life tailing logs - check out loki

# security

* need to provide a root dir which will be bind mounted, rest of fs is readonly

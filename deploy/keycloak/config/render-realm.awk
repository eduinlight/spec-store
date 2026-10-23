{
  rendered = ""
  rest = $0
  while (match(rest, /\$\{[A-Z_][A-Z0-9_]*(:[^}]*)?\}/)) {
    token = substr(rest, RSTART + 2, RLENGTH - 3)
    name = token
    fallback = ""
    split_at = index(token, ":")
    if (split_at > 0) {
      name = substr(token, 1, split_at - 1)
      fallback = substr(token, split_at + 1)
    } else if (!(name in ENVIRON)) {
      missing[name] = 1
    }
    value = (name in ENVIRON) ? ENVIRON[name] : fallback
    rendered = rendered substr(rest, 1, RSTART - 1) value
    rest = substr(rest, RSTART + RLENGTH)
  }
  print rendered rest
}
END {
  for (name in missing) {
    print "render-realm: " name " is not set" > "/dev/stderr"
    failed = 1
  }
  exit failed
}

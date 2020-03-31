# bencode-decode
<a href="http://www.wtfpl.net/"><img
       src="http://www.wtfpl.net/wp-content/uploads/2012/12/wtfpl-badge-4.png"
       width="80" height="15" alt="WTFPL" /></a>

Minimal, lean, no-bullshit, iterator-based [bencode](https://wiki.theory.org/index.php/BitTorrentSpecification#Bencoding) decoder.

```rust
use bencode_decode::{Parser, decode};
use std::fs::File;

let f = File::open("./test/ubuntu-18.04.4-live-server-amd64.iso.torrent").unwrap();
let mut parser = Parser::new(f);
let res = decode(&mut parser, None).unwrap();
println!("Your torrent file in its raw glory: {:?}", res);
```

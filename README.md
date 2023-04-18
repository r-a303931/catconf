# Catconf

For when you want:

1.  Runtime configuration for after the binary is compiled
2.  A single file binary

This library allows for taking the final result binary, and just concatenating the configuration to the end:

    cat target/debug/binary <(echo -n "CATCONF") conf > confedbinary

Great, but how to get the configuration back out and use it in the code? catconf!

It&rsquo;s use is pretty simple:

    use catconf::ConfReaderOptions;

    let conf_reader = ConfReaderOptions::new(b"CATCONF".to_vec()).read_from_exe()?;

This returns a `Vec<u8>` which can be transformed further, by converting to UTF-8 and combined with Serde, decompressing with zlib, etc.

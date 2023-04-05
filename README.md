dnsmaster
=========

`dnsmaster` is a small tool to update DNS records to the latest IP address of
your server. It essentially is a DDNS client but without explicitly requiring
that your host supports DDNS. If there's a public API, `dnsmaster` can use it.


## Usage

To run `dnsmaster` forever and let it monitor your IP, simply run the
following:

```sh
dnsmaster example.com
```

## License

This software is licensed under the MIT license


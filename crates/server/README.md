# Server 

## HTTP

## QUIC

## TCP
Implementation of TCP based on [RFC 9293](https://datatracker.ietf.org/doc/html/rfc9293)

## UDP

## Utils 

Almost all of the code from the utils crate in order to generate the tuntap interaface is taken from [smoltcp](https://github.com/smoltcp-rs/smoltcp)
This was done in order to take what we needed from there while still maintaining the ability to easily be no std. Documentation of any of the code should not be taken as correct or accurate, but rather as my own understanding of what is happening throughout these fuctions. Again this project is meant as a learning experience at the moment and should be treated as such. 
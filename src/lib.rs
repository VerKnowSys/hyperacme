#![warn(clippy::all)]
//! acme-lib is a library for accessing ACME (Automatic Certificate Management Environment)
//! services such as [Let's Encrypt](https://letsencrypt.org/).
//!
//! Uses ACME v2 to issue/renew certificates.
//!
//! # Example
//!
//! ```no_run
//! use hyperacme::{Error, Certificate, Directory, DirectoryUrl};
//! use hyperacme::create_p384_key;
//! use std::time::Duration;
//!
//! async fn request_cert() -> Result<Certificate, Error> {
//!
//! // Use DirectoryUrl::LetsEncrypStaging for dev/testing.
//! let url = DirectoryUrl::LetsEncrypt;
//!
//! // Create a directory entrypoint.
//! let dir = Directory::from_url(url).await?;
//!
//! // Your contact addresses, note the `mailto:`
//! let contact = vec!["mailto:foo@bar.com".to_string()];
//!
//! // Generate a private key and register an account with your ACME provider.
//! // You should write it to disk any use `load_account` afterwards.
//! let acc = dir.register_account(contact.clone()).await?;
//!
//! // Example of how to load an account from string:
//! let privkey = acc.acme_private_key_pem().await?;
//! let acc = dir.load_account(&privkey, contact).await?;
//!
//! // Order a new TLS certificate for a domain.
//! let mut ord_new = acc.new_order("example.com", &[]).await?;
//!
//! // If the ownership of the domain(s) have already been
//! // authorized in a previous order, you might be able to
//! // skip validation. The ACME API provider decides.
//! let ord_csr = loop {
//!     // are we done?
//!     if let Some(ord_csr) = ord_new.confirm_validations().await {
//!         break ord_csr;
//!     }
//!
//!     // Get the possible authorizations (for a single domain
//!     // this will only be one element).
//!     let auths = ord_new.authorizations().await?;
//!
//!     // For HTTP, the challenge is a text file that needs to
//!     // be placed in your web server's root:
//!     //
//!     // /var/www/.well-known/acme-challenge/<token>
//!     //
//!     // The important thing is that it's accessible over the
//!     // web for the domain(s) you are trying to get a
//!     // certificate for:
//!     //
//!     // http://example.com/.well-known/acme-challenge/<token>
//!     let chall = auths[0].http_challenge().await.unwrap();
//!
//!     // The token is the filename.
//!     let token = chall.http_token().await;
//!     let path = format!(".well-known/acme-challenge/{}", token);
//!
//!     // The proof is the contents of the file
//!     let proof = chall.http_proof().await?;
//!
//!     // Here you must do "something" to place
//!     // the file/contents in the correct place.
//!     // update_my_web_server(&path, &proof);
//!
//!     // After the file is accessible from the web, the calls
//!     // this to tell the ACME API to start checking the
//!     // existence of the proof.
//!     //
//!     // The order at ACME will change status to either
//!     // confirm ownership of the domain, or fail due to the
//!     // not finding the proof. To see the change, we poll
//!     // the API with 5000 milliseconds wait between.
//!     chall.validate(Duration::from_millis(5000)).await?;
//!
//!     // Update the state against the ACME API.
//!     ord_new.refresh().await?;
//! };
//!
//! // Ownership is proven. Create a private key for
//! // the certificate. These are provided for convenience, you
//! // can provide your own keypair instead if you want.
//! let pkey_pri = create_p384_key()?;
//!
//! // Submit the CSR. This causes the ACME provider to enter a
//! // state of "processing" that must be polled until the
//! // certificate is either issued or rejected. Again we poll
//! // for the status change.
//! let ord_cert =
//!     ord_csr.finalize_pkey(pkey_pri, Duration::from_millis(5000)).await?;
//!
//! // Now download the certificate. Also stores the cert in
//! // the persistence.
//! let cert = ord_cert.download_cert().await?;
//! println!("{:?}", cert);
//!
//! Ok(cert)
//! }
//! ```
//!
//! ## Domain ownership
//!
//! Most website TLS certificates tries to prove ownership/control over the domain they
//! are issued for. For ACME, this means proving you control either a web server answering
//! HTTP requests to the domain, or the DNS server answering name lookups against the domain.
//!
//! To use this library, there are points in the flow where you would need to modify either
//! the web server or DNS server before progressing to get the certificate.
//!
//! See [`http_challenge`] and [`dns_challenge`].
//!
//! ### Multiple domains
//!
//! When creating a new order, it's possible to provide multiple alt-names that will also
//! be part of the certificate. The ACME API requires you to prove ownership of each such
//! domain. See [`authorizations`].
//!
//! [`http_challenge`]: order/struct.Auth.html#method.http_challenge
//! [`dns_challenge`]: order/struct.Auth.html#method.dns_challenge
//! [`authorizations`]: order/struct.NewOrder.html#method.authorizations
//!
//! ## Rate limits
//!
//! The ACME API provider Let's Encrypt uses [rate limits] to ensure the API i not being
//! abused. It might be tempting to put the `delay` really low in some of this
//! libraries' polling calls, but balance this against the real risk of having access
//! cut off.
//!
//! [rate limits]: https://letsencrypt.org/docs/rate-limits/
//!
//! ### Use staging for dev!
//!
//! Especially take care to use the Let`s Encrypt staging environment for development
//! where the rate limits are more relaxed.
//!
//! See [`DirectoryUrl::LetsEncryptStaging`].
//!
//! [`DirectoryUrl::LetsEncryptStaging`]: enum.DirectoryUrl.html#variant.LetsEncryptStaging
//!
//! ## Implementation details
//!
//! The library tries to pull in as few dependencies as possible. (For now) that means using
//! synchronous I/O and blocking cals. This doesn't rule out a futures based version later.
//!
//! It is written by following the
//! [ACME draft spec 18](https://tools.ietf.org/html/draft-ietf-acme-acme-18), and relies
//! heavily on the [openssl](https://docs.rs/openssl/) crate to make JWK/JWT and sign requests
//! to the API.
//!

#[cfg(doctest)]
doc_comment::doctest!("../README.md");

mod acc;
mod cert;
mod dir;
mod error;
mod jwt;
mod req;
mod trans;
mod util;

pub mod api;
pub mod order;

#[macro_use]
extern crate tracing;

#[cfg(test)]
mod test;

pub use crate::acc::{Account, RevocationReason};
pub use crate::cert::{create_p256_key, create_p384_key, create_rsa_key, Certificate};
pub use crate::dir::{Directory, DirectoryUrl};
pub use crate::error::Error;

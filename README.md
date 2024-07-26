Run a web server that displays the path a Nixpkgs pull request will
take through the various release channels.  An instance is available
at the time of writing at <https://tracker.lel.lol/>.


Installation
------------

Build and runtime dependencies:
 - libsystemd
 - OpenSSL

Other build dependencies:
 - Cargo
 - rustc
 - pkg-config

Other runtime dependencies:
 - Git

In most cases, building should be as simple as

	cargo run


Usage
-----

To see usage run:

```sh
./pr-tracker --help
```

The following environment variables are expected:
| Name  | Usage  |
|---|---|
|PR_TRACKER_GITHUB_TOKEN   | A github access token to access the github graphql api.  |
|PR_TRACKER_MAIL_PASSWD   | The password to use for secure email sending.  |

pr-tracker expects the socket(s) for it to listen on to be set up for
it by a service supervisor, using the systemd socket activation
protocol.  It does not support binding its own sockets.  To run
outside of a systemd unit, you can use systemd-socket-activate:

	systemd-socket-activate -l 0.0.0.0:8000 pr-tracker [...]

Further information on available command line arguments can be
obtained with

Scripts
-----

`scripts/` contains tampermonkey scripts to add buttons directly to github for tracking/subscribing to pull-requests.

Development
-----------

The upstream git repository for pr-tracker is available at
<https://github.com/patrickdag/pr-tracker/>.

License
-------

Copyright 2021 Alyssa Ross <hi@alyssa.is>

This program is free software; you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as
published by the Free Software Foundation; either version 3 of the
License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but
WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public
License along with this program; if not, see
<https://www.gnu.org/licenses>.

Additional permission under GNU AGPL version 3 section 7

If you modify this Program, or any covered work, by linking or
combining it with OpenSSL (or a modified version of that library),
containing parts covered by the terms of the OpenSSL License, or the
Original SSLeay License, the licensors of this Program grant you
additional permission to convey the resulting work.

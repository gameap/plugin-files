# Node image for the E2E workflow: a systemd-booted container that reproduces a
# real machine closely enough for two things that a plain container cannot do.
# gameapctl takes its systemd code paths instead of the fork fallback it uses
# when PID 1 is not systemd, and install-files-linux.sh finds the systemctl it
# refuses to run without.
ARG BASE_IMAGE=debian:13
FROM ${BASE_IMAGE}

ARG DEBIAN_FRONTEND=noninteractive
ENV container=docker

# ca-certificates has to be present before gameapctl runs: when the installer
# has to install it itself, it stops and asks to be re-run instead of
# continuing. iproute2 and procps back the node fixture's listeningPorts().
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      systemd \
      systemd-sysv \
      dbus \
      ca-certificates \
      curl \
      gnupg \
      tar \
      xz-utils \
      iproute2 \
      procps \
 && rm -rf /var/lib/apt/lists/*

# Masking is done with symlinks rather than `systemctl mask` because there is
# no systemd manager to talk to during the image build.
#
# udevd/modules-load have no hardware to work with; networkd-wait-online would
# hold up network-online.target for up to 2 minutes; resolved would fight
# docker over /etc/resolv.conf; apparmor.service must not load container
# profiles into the kernel shared with the host under --privileged; getty keeps
# failed units out of `systemctl is-system-running`.
RUN for unit in \
      systemd-udevd.service \
      systemd-udevd-kernel.socket \
      systemd-udevd-control.socket \
      systemd-modules-load.service \
      systemd-networkd-wait-online.service \
      systemd-resolved.service \
      apparmor.service \
      getty.target \
      console-getty.service \
    ; do \
      ln -sf /dev/null "/etc/systemd/system/${unit}"; \
    done

STOPSIGNAL SIGRTMIN+3

CMD ["/sbin/init"]

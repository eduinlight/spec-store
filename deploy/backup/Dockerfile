FROM postgres:18
RUN apt-get update \
  && apt-get install -y --no-install-recommends rclone ca-certificates \
  && rm -rf /var/lib/apt/lists/* \
  && mkdir -p /etc/rclone && touch /etc/rclone/rclone.conf
ENV RCLONE_CONFIG=/etc/rclone/rclone.conf
COPY deploy/backup/backup.sh deploy/backup/restore-check.sh /usr/local/bin/
USER postgres
WORKDIR /tmp
ENTRYPOINT ["/usr/local/bin/backup.sh"]

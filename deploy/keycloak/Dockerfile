FROM quay.io/keycloak/keycloak:26.8.1 AS builder
ENV KC_DB=postgres \
    KC_HEALTH_ENABLED=true \
    KC_METRICS_ENABLED=true
COPY deploy/keycloak/themes/sdd-vault /opt/keycloak/themes/sdd-vault
COPY packages/ui/src/styles/tokens.css /opt/keycloak/themes/sdd-vault/login/resources/css/tokens.css
RUN /opt/keycloak/bin/kc.sh build

FROM quay.io/keycloak/keycloak:26.8.1
COPY --from=builder /opt/keycloak/ /opt/keycloak/
COPY deploy/keycloak/realm/sdd-vault.json /opt/keycloak/data/realm/sdd-vault.json
COPY deploy/keycloak/config/render-realm.awk /opt/keycloak/data/realm/render-realm.awk
ENTRYPOINT ["/opt/keycloak/bin/kc.sh"]
CMD ["start", "--optimized"]

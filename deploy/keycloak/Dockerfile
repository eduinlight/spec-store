FROM quay.io/keycloak/keycloak:26.8.1 AS builder
ENV KC_DB=postgres \
    KC_HEALTH_ENABLED=true \
    KC_METRICS_ENABLED=true
COPY deploy/keycloak/themes/spec-store /opt/keycloak/themes/spec-store
COPY packages/ui/src/styles/tokens.css /opt/keycloak/themes/spec-store/login/resources/css/tokens.css
RUN /opt/keycloak/bin/kc.sh build

FROM quay.io/keycloak/keycloak:26.8.1
COPY --from=builder /opt/keycloak/ /opt/keycloak/
COPY deploy/keycloak/realm/spec-store.json /opt/keycloak/data/realm/spec-store.json
COPY deploy/keycloak/config/render-realm.awk /opt/keycloak/data/realm/render-realm.awk
ENTRYPOINT ["/opt/keycloak/bin/kc.sh"]
CMD ["start", "--optimized"]

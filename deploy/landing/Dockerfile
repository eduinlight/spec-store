FROM nginx:1.27-alpine
COPY deploy/landing.conf /etc/nginx/conf.d/default.conf
COPY apps/landing/build/client /usr/share/nginx/html
EXPOSE 80

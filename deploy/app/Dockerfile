FROM nginx:1.27-alpine
COPY deploy/spa.conf /etc/nginx/conf.d/default.conf
COPY apps/app/dist /usr/share/nginx/html
EXPOSE 80

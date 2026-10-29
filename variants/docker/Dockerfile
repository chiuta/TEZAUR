FROM nginx:alpine
LABEL org.opencontainers.image.source="https://github.com/chiuta/TEZAUR" org.opencontainers.image.licenses="MIT"
COPY site/ /usr/share/nginx/html/
COPY nginx.conf /etc/nginx/conf.d/default.conf

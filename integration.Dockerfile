FROM projectserum/build:v0.17.0
ARG test_name

COPY / /hubble
WORKDIR /hubble

RUN yarn
RUN ./integration.sh $test_name
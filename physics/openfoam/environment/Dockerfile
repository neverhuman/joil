FROM ubuntu:24.04@sha256:f610ab94648195aa356059f5b41d6085c9d4d903c072430cdd1af7bdb646106b
ENV DEBIAN_FRONTEND=noninteractive OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl gpg python3 && rm -rf /var/lib/apt/lists/*
COPY openfoam.asc /usr/share/keyrings/openfoam.asc
RUN echo "deb [signed-by=/usr/share/keyrings/openfoam.asc] http://dl.openfoam.org/ubuntu noble main" > /etc/apt/sources.list.d/joil-openfoam.list && apt-get update && cd /tmp && apt-get download openfoam14=20261008 && echo "99fe066c827f2834474692d63c2a3c45ef094a7253d115841635ba35acbb0067  openfoam14_20261008_amd64.deb" | sha256sum -c - && apt-get install -y --no-install-recommends ./openfoam14_20261008_amd64.deb && rm -f /tmp/openfoam14_20261008_amd64.deb && rm -rf /var/lib/apt/lists/*
SHELL ["/bin/bash", "-c"]
RUN source /opt/openfoam14/etc/bashrc && foamRun -help && blockMesh -help
WORKDIR /work
ENTRYPOINT ["/bin/bash", "-c", "source /opt/openfoam14/etc/bashrc; exec \"$@\"", "joil"]

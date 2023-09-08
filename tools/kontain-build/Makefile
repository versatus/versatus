PREFIX = /opt/vrrb
BUILD_DIR = ./build

KM_URL = https://github.com/kontainapp/km.git
KM_TAG = v0.9.8

CRUN_URL = https://github.com/kontainapp/crun.git
#CRUN_TAG = 1.3
CRUN_BRANCH = krun
CRUN_PREFIX = ${PREFIX}

default: ${BUILD_DIR}/runtime.tgz

${BUILD_DIR}/runtime.tgz: ${BUILD_DIR}/pkg/bin/crun ${BUILD_DIR}/pkg/bin/km ${BUILD_DIR}/pkg/_MANIFEST
	tar zcf ${BUILD_DIR}/runtime.tgz -C ${BUILD_DIR}/pkg .

${BUILD_DIR}/pkg/bin:
	mkdir -p $@

${BUILD_DIR}/pkg/_MANIFEST:	${BUILD_DIR}/pkg/bin
	# This is a placeholder til the IPLD pacakging has been developed
	cd $< ; sha256sum km crun > ../_MANIFEST

${BUILD_DIR}:
	mkdir -p $@

${BUILD_DIR}/km-src:	${BUILD_DIR}
	rm -rf $@; git clone ${KM_URL} $@

${BUILD_DIR}/km-src/build/km/km:	${BUILD_DIR}/km-src
	cd $< && git checkout ${KM_TAG} && \
		make -C km

${BUILD_DIR}/pkg/bin/km: ${BUILD_DIR}/km-src/build/km/km ${BUILD_DIR}/pkg/bin
	cp -a $< $@

${BUILD_DIR}/crun-src:	${BUILD_DIR}
	rm -rf $@; git clone ${CRUN_URL} $@

${BUILD_DIR}/crun-src/crun: ${BUILD_DIR}/crun-src
	cd $< ; \
		git checkout ${CRUN_BRANCH} && \
		./autogen.sh && \
		./configure --prefix=${CRUN_PREFIX}
	make -C $<

${BUILD_DIR}/pkg/bin/crun: ${BUILD_DIR}/crun-src/crun ${BUILD_DIR}/pkg/bin
	cp -a $< $@

clean:
	rm -rf ${BUILD_DIR}

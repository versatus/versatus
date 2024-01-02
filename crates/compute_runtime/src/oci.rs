//! The oci module contains common code for managing jobs with an OCI-compliant runtime.

use anyhow::{anyhow, Context, Result};
use derive_builder::Builder;
use log::{debug, info};
use oci_spec::runtime::{Process, ProcessBuilder, Root, RootBuilder, Spec};
use std::collections::HashMap;
use std::fs::File;
use std::fs::{copy, create_dir};
use std::io::Read;
use std::os::unix::io::FromRawFd;
use std::os::unix::net::UnixListener;
use std::process::Command;
use std::str;
use std::thread;
use telemetry::request_stats::RequestStats;
use uds::{UnixListenerExt, UnixSocketAddr, UnixStreamExt};

/// The directory under the temporary tree where we build the container's root filesystem.
/// Interestingly, it seems as though regardless of what we set this to in the config.json spec
/// file, some OCI runtimes always insist that it be the string 'rootfs'...
const CONTAINER_ROOT: &str = "rootfs";

/// Wrap RequestStats and provide a default to satisfy derive_builder used below.
struct OciStats(RequestStats);
impl Default for OciStats {
    fn default() -> Self {
        OciStats::new()
    }
}

impl OciStats {
    pub fn new() -> Self {
        info!("Building new stats collector");
        OciStats {
            0: RequestStats::new("OciManager".to_string(), "oci-exec".to_string())
                .expect("Failed to create stats collector"),
        }
    }
}

/// OciManager provides functionality for building and managing container execution using an
/// OCI-compliant runtime.
#[derive(Builder)]
pub struct OciManager {
    /// The directory root (bundle dir) under which we'll build and execute the container. We don't
    /// clean up automatically, and create a number of subdirectories for building/executing the
    /// container.
    runtime_path: String,
    /// This is the path to the OCI-compliant container runtime (eg, runc, crun, krun, youki, etc).
    oci_runtime: String,
    /// The vector of command line arguments for the container payload (ie, the program to
    /// execute).
    container_payload: Vec<String>,
    /// The container ID. Will likely be a job UUID passed in by the caller. Should be unique.
    container_id: String,
    /// The hostname to be assigned to this container.
    hostname: String,
    /// The domain name to be assigned to this container.
    domainname: String,
    /// A map of key/value strings representing some additional optional annotations for the
    /// container.
    annotations: HashMap<String, String>,
    /// The internal representation of the container configuration.
    #[builder(setter(skip = true))]
    oci_config: Option<Spec>,
    /// object for tracing timing of phases of execution
    #[builder(setter(skip = true))]
    stats: OciStats,
}

impl OciManager {
    /// Returns the container root filesystem directory.
    pub fn rootfs(&self) -> String {
        format!("{}/{}", self.runtime_path, CONTAINER_ROOT)
    }
    /// Prep the container manager temporary directory by creating directories, etc.
    pub fn prep(&mut self) -> Result<()> {
        // First, create all of the sub directories we'll need to build and run an OCI container.
        self.stats.0.start("setup".to_string())?;
        debug!(
            "Creating container rootfs under: {}/{}",
            &self.runtime_path, CONTAINER_ROOT
        );
        create_dir(format!("{}/{}", self.runtime_path, CONTAINER_ROOT)).context("rootfs")?;
        let subdirs = ["root", "tmp", "diag", "sbin", "dev", "bin"];
        for subdir in subdirs.iter() {
            let path = format!("{}/{}/{}", self.runtime_path, CONTAINER_ROOT, subdir);
            create_dir(&path).context("subdir")?;
        }

        // TODO: once the package stuff exists and the caller passes in the details, we'll replace
        // these placeholders.
        let _ret = copy(
            "/usr/bin/busybox",
            format!("{}/{}/bin/sh", self.runtime_path, CONTAINER_ROOT),
        )
        .context("shell")?;
        let _ret = copy(
            "/usr/bin/busybox",
            format!("{}/{}/bin/busybox", self.runtime_path, CONTAINER_ROOT),
        )
        .context("busybox")?;
        self.stats.0.stop("setup".to_string())?;
        Ok(())
    }

    /// Generate a default configuration for this OCI runtime and modify it with the specified
    /// customisations.
    pub fn spec(&mut self) -> Result<()> {
        // run container runtime with `spec` option and parse spec
        self.stats.0.start("spec".to_string())?;
        let cmd = Command::new(&self.oci_runtime)
            .arg("spec")
            .arg("--rootless") // not all runtimes seem to support rootless
            .arg("--bundle")
            .arg(&self.runtime_path)
            .current_dir(&self.runtime_path)
            .output()
            .context(self.oci_runtime.to_string())?;

        debug!(
            "Spec generation errors: {}",
            str::from_utf8(&cmd.stderr).context("Retreving stderr")?
        );
        debug!(
            "Spec generation output: {}",
            str::from_utf8(&cmd.stdout).context("Retreving stdout")?
        );

        // Read generated config/spec file
        let spec_path = format!("{}/config.json", &self.runtime_path);

        // Modify generated config/spec to caller's taste
        let mut oci_config = Spec::load(spec_path.clone()).context("spec file")?;
        oci_config
            .set_hostname(Some(self.hostname.clone()))
            .set_annotations(Some(self.annotations.clone()))
            .set_domainname(Some(self.domainname.clone()));

        // Modify the Process object, responsible for (including others) the command line arguments
        // to be passed to exec*().
        let mut proc: Process;
        match oci_config.process() {
            None => proc = ProcessBuilder::default().build()?,
            Some(genproc) => proc = genproc.to_owned(),
        }
        proc.set_args(Some(self.container_payload.to_owned()));
        let guest_env: Vec<String> = vec!["PATH=/bin".to_string(), "LOCATION=sfo".to_string()];
        proc.set_env(Some(guest_env.to_owned()));
        oci_config.set_process(Some(proc));

        // Modify the Root object, which gives the detail about the root file system
        let mut rootfs: Root;
        match oci_config.root() {
            None => rootfs = RootBuilder::default().build()?,
            Some(genroot) => rootfs = genroot.to_owned(),
        }
        rootfs.set_path(std::path::PathBuf::from(format!(
            "{}/{}",
            self.runtime_path, CONTAINER_ROOT
        )));
        // We currently don't mark the root filesystem read-only, but want to. In the short term,
        // we're only running WASM modules in production, which (due to our WASI constraints) means
        // that the guest workload can't read/write the root filesystem anyway. We currently use
        // this as a place to store diagnostic logs -- especially during testing. What would be
        // better is allocating a small (32MB) tmpfs (zramfs?) volume for such things and making that the
        // only writable volume.
        rootfs.set_readonly(Some(false));
        oci_config.set_root(Some(rootfs));

        // We can manipulate other container runtime parameters here in the future. I suspect that
        // the default resource limits and namespaces, may not be optimal out of the box. Also, in
        // order to support rootless containers/VMs, we'll likely need to adjust quite a bit.

        // Stash our modified config object ready to be written out before we build and execute.
        self.oci_config = Some(oci_config.to_owned());
        debug!("Parsed OCI config: {:?}", &self.oci_config);
        self.stats.0.stop("spec".to_string())?;
        Ok(())
    }

    /// Executes a prepped OCI-compliant container
    pub fn execute(&mut self) -> Result<()> {
        // First, write out our configuration file over the default one generated earlier.
        self.stats.0.start("exec".to_string())?;
        match &self.oci_config {
            None => return Err(anyhow!("Attempted to run empty container spec")),
            Some(spec) => {
                spec.save(format!("{}/config.json", &self.runtime_path))
                    .context("Write container spec")?;
            }
        }

        // We name a unix domain socket, and in a thread, create it, listen on it and read special
        // magic messages that will contain a Linux/Solaris kernel file descriptor that is open and
        // being shared with us. The thread then converts that file descriptor to a Rust file
        // handle and reads from it. The container runtime process is the creator of the
        // shared file descriptor and attaches the guest's stdout to it (no stderr, for shame).
        // Essentially the data that the thread is reading is stdout for the container's payload.
        let console_socket = format!("{}/console.sock", &self.runtime_path);
        debug!("Using console socket path: {}", &console_socket);
        let sock = console_socket.clone();
        let con_thread = thread::spawn(move || runtime_output(sock));

        // XXX: Note that because we have access to /dev/kvm (and some other magic?) we're able to
        // run both Kontain and OCI runc without being root.
        let job = Command::new(&self.oci_runtime)
            .arg("run")
            //.arg("--detach") // TODO: not all runtimes will want or need this and it leaves the
            //container around afterwards
            .arg("--bundle")
            .arg(&self.runtime_path)
            .arg("--console-socket")
            .arg(&console_socket)
            .arg(&self.container_id)
            .output()
            .context("OCI container exec")?;
        // If the container runtime fails to start, these will contain details of that. Once the
        // container runtime starts, its guest stdio comes over the file descriptors shared over
        // the socket and collected in the parallel thread.
        match job.status.code() {
            Some(0) => {}
            _ => {
                info!("Container runtime exec failed");
                info!(
                    "Stdout: {}",
                    str::from_utf8(&job.stdout).context("Retreving stdout")?
                );
                info!(
                    "Stderr: {}",
                    str::from_utf8(&job.stderr).context("Retreving stderr")?
                );
                return Err(anyhow!("Container runtime exec failed"));
            }
        }

        let tret = con_thread.join().expect("Thread panic");
        // TODO: When the caller exists, pass this back up to them.
        info!("Thread output: {}", tret);

        self.stats.0.stop("exec".to_string())?;
        Ok(())
    }
}

/// Private function for handling container runtime output over shared file descriptors.
fn runtime_output(console_socket: String) -> String {
    // We need to create a Unix domain socket and listen on it to receive the file handle(s)
    // passed back to us by the OCI runtime to represent the pseudoterminal (PTY) attached to
    // the container's stdio.
    let addr = UnixSocketAddr::new(&console_socket).expect("Console socket");
    let listener = UnixListener::bind_unix_addr(&addr).expect("Console socket listener");
    let (conn, _peer) = listener.accept_unix_addr().expect("Console socket accept");
    // Receive the file handle(s)
    let mut fds = [-1, 3];
    let (_, num_fds) = conn
        .recv_fds(&mut [0u8; 3], &mut fds)
        .expect("Console socket file descriptors");
    debug!(
        "Received {} file descriptors ({:?}) over socket {}",
        num_fds, fds, console_socket
    );

    // Create Rust file handles from the raw Linux descriptor(s) passed back over the socket. We
    // only ever get one file descriptor back from the container runtime. This file handle
    // represents stdout for the guest. The OCI runtime doesn't do us any favours with stderr at
    // all, so that is instead redirected to a file elsewhere and handled separately. This ought to
    // be OK for our use case(s) though.
    let mut f = unsafe { File::from_raw_fd(fds[0]) };

    // Retrieve the job output from the magic filehandle passed to us from the container
    // runtime over the console socket. We get the data back a line at a time as a set of bytes.
    let mut output: Vec<u8> = vec![0; 8192];
    let mut text: String = String::new();
    while let Ok(count) = f.read(&mut output) {
        let line = str::from_utf8(&output[0..count]).expect("Converting output to UTF8");
        debug!("Payload output: {}", line);
        text += line;
    }

    // Thread returns all output as a single string.
    text
}

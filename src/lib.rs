use libc;
use std::{
    cell::Cell,
    fmt,
    path::{Path, PathBuf},
    rc::Rc,
};

const NR_JOBS: usize = 20;
const PATH_BUFSIZE: usize = 1024;
const COMMAND_BUFSIZE: usize = 1024;
const TOKEN_BUFSIZE: usize = 64;

pub struct Process {
    pub command:     Command,
    pub argc:        Cell<u8>, // argument count
    pub argv:        Cell<u8>, // argument value
    pub input_path:  PathBuf,
    pub output_path: PathBuf,
    pub pid:         usize,
    pub p_type:      usize,
    pub status:      Status,
    pub next:        Option<Box<Process>>,
}
struct Job {
    id:      usize,
    root:    Option<Box<Process>>,
    command: Command,
    pgid:    usize,
    mode:    Execution,
}

struct Shell {
    cur_user: &'static str,
    cur_dir:  PathBuf,
    pw_dir:   PathBuf,
    jobs:     Vec<Job>,
}

impl Shell {
    fn get_job_id_by_pid(&self, pid: usize) -> Option<usize> {
        for job in self.jobs.iter() {
            let mut proc = job.root.as_ref();
            while let Some(p) = proc {
                if p.pid == pid {
                    return Some(job.id);
                }
                proc = p.next.as_ref();
            }
        }
        None
    }

    fn get_job_by_id(&self, id: usize) -> Option<&Job> {
        self.jobs.get(id)
    }

    fn get_root_by_job_id(&self, id: usize) -> Option<&Box<Process>> {
        self.jobs.get(id).and_then(|j| j.root.as_ref())
    }

    fn get_pgid_by_job_id(&self, id: usize) -> Option<usize> {
        self.jobs.get(id).map(|j| j.pgid)
    }

    fn get_proc_count(&self, id: usize, filter: ProcFilter) -> Option<usize> {
        let mut count = 0;
        let mut proc = self.get_root_by_job_id(id);
        if proc.is_none() {
            return None;
        }

        while let Some(p) = proc {
            match (filter, p.status == Status::Done) {
                (ProcFilter::All, _)
                | (ProcFilter::Done, true)
                | (ProcFilter::Remaining, false) => {
                    count += 1;
                }
                _ => (),
            }
            proc = p.next.as_ref();
        }

        Some(count)
    }

    fn get_next_job_id(&self) -> Option<usize> {
        self.jobs.get(0).map(|j| j.id)
    }

    fn print_processes_of_job(&self, id: usize) -> Option<()> {
        let mut proc = self.jobs.get(id).and_then(|job| {
            println!("[{}]", job.id);
            job.root.as_ref()
        });
        if proc.is_none() {
            return None;
        }

        while let Some(p) = proc {
            println!(" {}", p.pid);
            proc = p.next.as_ref();
        }
        println!("\n");

        Some(())
    }

    fn print_job_status(self, id: usize) -> Result<(), &'static str> {
        let mut proc = self.jobs.get(id).and_then(|job| {
            println!("[{}]", job.id);
            job.root.as_ref()
        });
        if proc.is_none() {
            return Err("job id not found");
        }

        while let Some(p) = proc {
            println!("\t{}\t{}\t{}", p.pid, p.status, p.command);
            proc = p.next.as_ref();
            if proc.is_some() {
                println!("|\n");
            } else {
                println!("\n");
            }
        }

        Ok(())
    }

    fn release_job(self, id: usize) -> Result<(), &'static str> {
        let mut proc = self.jobs.get(id).and_then(|job| {
            println!("[{}]", job.id);
            job.root.as_ref()
        });
        if proc.is_none() {
            return Err("job id not found");
        }

        while let Some(p) = proc {
            println!("\t{}\t{}\t{}", p.pid, p.status, p.command);
            proc = p.next.as_ref();
            if proc.is_some() {
                println!("|\n");
            } else {
                println!("\n");
            }
        }

        Ok(())
    }
}

enum Execution {
    Background = 0,
    Foreground = 1,
    Pipeline = 2,
}

// #[derive(Clone, Copy)]

#[derive(PartialEq, Copy, Clone)]
enum ProcFilter {
    All = 0,
    Done = 1,
    Remaining = 2,
}

macro_rules! enum_val {
    ($($pub:vis enum $name:ident::<$type:ty>{
        $($variant:ident = $val:expr),*,
    })*) => {
        $(
            #[derive(Copy,Clone,PartialEq)]
            $pub enum $name {
                $($variant),*
            }

            impl $name {
                fn value(&self) -> $type {
                    match self {
                        $(Self::$variant => $val),*
                    }
                }
            }

            impl fmt::Display for $name {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    match self {
                        $($name::$variant => write!(f, stringify!($variant))),*
                    }
                }
        })*
    };
}

enum_val! {
    pub enum Command::<usize> {
        External = 0,
        Exit = 1,
        Cd = 2,
        Jobs = 3,
        Fg = 4,
        Bg = 5,
        Kill = 6,
        Export = 7,
        Unset = 8,
    }

    pub enum Status::<usize> {
        Running    = 0,
        Done       = 1,
        Suspended  = 2,
        Continued  = 3,
        Terminated = 4,
    }

    enum TokenDelimiters::<char> {
        Space    = ' ',
        Tab      = '\t',
        Carriage = '\r',
        Newline  = '\n',
        Alert    = '\x07',
    }

    pub enum Color::<&'static str> {
        Empty  = "\033[m",
        Red    = "\033[1,37,41m",
        Yellow = "\033[1,33m",
        Cyan   = "\033[0,36m",
        Green  = "\033[0,32,32m",
        Gray   = "\033[1,30m",
    }
}

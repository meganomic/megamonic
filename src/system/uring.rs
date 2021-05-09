use thiserror::Error;
use std::sync::atomic::*;

#[allow(dead_code,non_camel_case_types)]
pub enum IOOPS {
        IORING_OP_NOP,
        IORING_OP_READV,
        IORING_OP_WRITEV,
        IORING_OP_FSYNC,
        IORING_OP_READ_FIXED,
        IORING_OP_WRITE_FIXED,
        IORING_OP_POLL_ADD,
        IORING_OP_POLL_REMOVE,
        IORING_OP_SYNC_FILE_RANGE,
        IORING_OP_SENDMSG,
        IORING_OP_RECVMSG,
        IORING_OP_TIMEOUT,
        IORING_OP_TIMEOUT_REMOVE,
        IORING_OP_ACCEPT,
        IORING_OP_ASYNC_CANCEL,
        IORING_OP_LINK_TIMEOUT,
        IORING_OP_CONNECT,
        IORING_OP_FALLOCATE,
        IORING_OP_OPENAT,
        IORING_OP_CLOSE,
        IORING_OP_FILES_UPDATE,
        IORING_OP_STATX,
        IORING_OP_READ,
        IORING_OP_WRITE,
        IORING_OP_FADVISE,
        IORING_OP_MADVISE,
        IORING_OP_SEND,
        IORING_OP_RECV,
        IORING_OP_OPENAT2,
        IORING_OP_EPOLL_CTL,
        IORING_OP_SPLICE,
        IORING_OP_PROVIDE_BUFFERS,
        IORING_OP_REMOVE_BUFFERS,
        IORING_OP_TEE,
        IORING_OP_SHUTDOWN,
        IORING_OP_RENAMEAT,
        IORING_OP_UNLINKAT,

        /* this goes last, obviously */
        IORING_OP_LAST,
}

#[repr(C)]
#[derive(Default,Debug)]
struct io_uring_sqe {
        opcode: u8,         /* type of operation for this sqe */
        flags: u8,          /* IOSQE_ flags */
        ioprio: u16,         /* ioprio for the request */
        fd: i32,             /* file descriptor to do IO on */
        off: u64,    /* offset into file */
        addr: u64,   /* pointer to buffer or iovecs */

        len: u32,            /* buffer size or number of iovecs */
        union1: u32,
        /*union {
                __kernel_rwf_t  rw_flags;
                fsync_flags: u32,
                poll_events: u16    /* compatibility */
                poll32_events: u32  /* word-reversed for BE */
                sync_range_flags: u32,
                msg_flags: u32,
                timeout_flags: u32,
                accept_flags: u32,
                cancel_flags: u32,
                open_flags: u32,
                statx_flags: u32,
                fadvise_advice: u32,
                splice_flags: u32,
        },*/
        //user_data: u64,      /* data to be passed back at completion time */
        user_pid: u32,
        user_smaps: u32,
        union2: [u64; 3]
        /*union {
                struct {
                        /* pack this to avoid bogus arm OABI complaints */
                        union {
                                /* index into fixed buffers, if used */
                                buf_index: u16,
                                /* for grouped buffer selection */
                                buf_group: u16,
                        } __attribute__((packed));
                        /* personality to use, if used */
                        personality: u16
                        splice_fd_in: i32,
                }
                __pad2: [u64, 3],
        }*/
}

#[repr(C)]
#[derive(Default,Debug)]
struct io_uring_cqe {
    //user_data: u64,  /* sqe->data submission passed back */
    user_pid: u32,
    user_smaps: u32,
    res: i32,        /* result code for this event */
    flags: u32,
}

/*
 * Filled with the offset for mmap(2)
 */
 #[repr(C)]
 #[derive(Default,Debug)]
struct io_sqring_offsets {
        head: u32,
        tail: u32,
        ring_mask: u32,
        ring_entries: u32,
        flags: u32,
        dropped: u32,
        array: u32,
        resv1: u32,
        resv2: u64,
}

/*
 * sq_ring->flags
 */
#[repr(C)]
#[derive(Default,Debug)]
struct io_cqring_offsets {
        head: u32,
        tail: u32,
        ring_mask: u32,
        ring_entries: u32,
        overflow: u32,
        cqes: u32,
        flags: u32,
        resv1: u32,
        resv2: u64,
}

#[repr(C)]
#[derive(Default,Debug)]
struct io_uring_params {
    sq_entries: u32,
    cq_entries: u32,
    flags: u32,
    sq_thread_cpu: u32,
    sq_thread_idle: u32,
    features: u32,
    resv: [u32; 4],
    sq_off: io_sqring_offsets,
    cq_off: io_cqring_offsets
}

// Default queue depth
const QUEUE_DEPTH: usize = 500;

// MMAP settings
const PROT_READ: u32 = 0x1;
const PROT_WRITE: u32 = 0x2;
const MAP_SHARED: u32 = 0x01;
const MAP_POPULATE: u32 = 0x008000;

// Magic offsets
const IORING_OFF_SQES: u64 = 0x10000000;
const IORING_OFF_SQ_RING: u64 = 0;


#[derive(Error, Debug)]
pub enum UringError {
    #[error("SYS_IO_URING_SETUP returned an error: {0}")]
    SysIoUringSetup(i32),

    #[error("SYS_MMAP 1 returned an error: {0}")]
    SysMmapSqPtr(i32),

    #[error("SYS_MMAP 2 returned an error: {0}")]
    SysMmapSqes(i32),

    #[error("Kernel version is not supported")]
    NotSupported,

    #[error("The CQ buffer is empty!")]
    ReadFromCqEmptyBuffer,

    #[error("SYS_IO_URING_ENTER returned an error: {0}")]
    SubmitToSqResult(i32),

    #[error("All jobs have been processed")]
    JobComplete,

}


macro_rules! checkerr {
    ($comparison:expr, $retval:expr) => {
        if $comparison {
            return Err($retval);
        }
    }
}

#[allow(dead_code)]
pub struct Uring {
    // How many entries the ring buffer has
    pub entries: usize,

    /** Keep track of submissions/completions **/

    // Total amount of CQEs read
    read_total: u64,

    // Amount of SQEs to submit when submit() is called
    submit: u32,

    // Total amount of SQEs submitted
    submit_total: u64,


    /** low level io_uring stuff **/
    sq_ptr: usize,
    cq_ptr: usize,

    sqes: *mut io_uring_sqe,
    cqes: *const io_uring_cqe,

    io_params: io_uring_params,

    // Useful stuff
    sring_tail: *mut AtomicU32,
    sring_mask: *const u32,
    sring_array: *mut u32,
    cring_head: *mut AtomicU32,
    cring_tail: *const u32,
    cring_mask: *const u32,

    ring_fd: i32,
}

// Since this is only used in a single thread just treat it as safe.
unsafe impl Send for Uring {}

impl Uring {
    pub fn new(mut queue_depth: usize) -> Result<Self, UringError> {
        let mut io_params = io_uring_params::default();

        if queue_depth == 0 {
            queue_depth = QUEUE_DEPTH
        }

        let ring_fd: i32;
        unsafe {
            asm!("syscall",
                in("rax") 425, //SYS_IO_URING_SETUP
                in("rdi") queue_depth,
                in("rsi") &mut io_params as *mut io_uring_params,
                out("rcx") _,
                out("r11") _,
                lateout("rax") ring_fd,
            );
        }

        checkerr!(ring_fd.is_negative(), UringError::SysIoUringSetup(ring_fd));

        // Check for kernel support for automatic CQE
        checkerr!((io_params.features & (1u32 << 0)) != 1, UringError::NotSupported);

        let mut sring_sz = io_params.sq_off.array + io_params.sq_entries * 4;

        let cring_sz = io_params.cq_off.cqes + io_params.cq_entries * std::mem::size_of::<io_uring_cqe>() as u32;

        // Whichever is bigger is the size we want
        sring_sz = sring_sz.max(cring_sz);

        // Ringbuffer mmap
        let ret: i64;
        unsafe {
            asm!("syscall",
                in("rax") 9, // SYS_MMAP
                in("rdi") 0, // address
                in("rsi") sring_sz, // length
                in("rdx") PROT_READ | PROT_WRITE, // prot
                in("r10") MAP_SHARED | MAP_POPULATE, // flags
                in("r8") ring_fd, // fd
                in("r9") IORING_OFF_SQ_RING, // offset
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret,
            );
        }

        checkerr!(ret.is_negative(), UringError::SysMmapSqPtr(ret as i32));

        let sq_ptr = ret as usize;

        let cq_ptr = sq_ptr;

        /* Save useful fields for later easy reference */
        let sring_tail = (sq_ptr as usize + io_params.sq_off.tail as usize) as *mut AtomicU32;
        let sring_mask = (sq_ptr as usize + io_params.sq_off.ring_mask as usize) as *const u32;
        let sring_array = (sq_ptr as usize + io_params.sq_off.array as usize) as *mut u32;


        // SQE array mmap
        let ret: i64;
        unsafe {
            asm!("syscall",
                in("rax") 9, // SYS_MMAP
                in("rdi") 0, // address
                in("rsi") io_params.sq_entries * std::mem::size_of::<io_uring_sqe>() as u32, // length
                in("rdx") PROT_READ | PROT_WRITE, // prot
                in("r10") MAP_SHARED | MAP_POPULATE, // flags
                in("r8") ring_fd, // fd
                in("r9") IORING_OFF_SQES, // offset
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret,
            );
        }

        checkerr!(ret.is_negative(), UringError::SysMmapSqes(ret as i32));

        let sqes = ret as *mut io_uring_sqe;


        /* Save useful fields for later easy reference */
        let cring_head = (cq_ptr as usize + io_params.cq_off.head as usize) as *mut AtomicU32;
        let cring_tail = (cq_ptr as usize + io_params.cq_off.tail as usize) as *const u32;
        let cring_mask = (cq_ptr as usize + io_params.cq_off.ring_mask as usize) as *const u32;

        let cqes = (cq_ptr as usize + io_params.cq_off.cqes as usize) as *const io_uring_cqe;


        Ok(Self {
            entries: queue_depth,
            read_total: 0,
            submit: 0,
            submit_total: 0,
            sq_ptr,
            cq_ptr,

            sqes,
            cqes,

            io_params,

            sring_tail,
            sring_mask,
            sring_array,
            cring_head,
            cring_tail,
            cring_mask,

            ring_fd,

        })
    }

    // Read next item from the completion queue
    pub fn read_from_cq(&mut self) -> Result<(i32, u32, u32), UringError> {
        // Load current head
        let mut head = unsafe { &*self.cring_head }.load(Ordering::Acquire);

        // If head == tail the buffer is empty
        checkerr!(head == unsafe { *self.cring_tail }, UringError::ReadFromCqEmptyBuffer);

        // Get CQE entry
        let cqe = unsafe { &*self.cqes.offset((head & (*self.cring_mask)) as isize) };

        // Save data so we can return it
        let res = cqe.res;
        let user_pid = cqe.user_pid;
        let user_smaps = cqe.user_smaps;

        // Increase head and save it
        head += 1;
        unsafe { &*self.cring_head }.store(head, Ordering::Release);


        // Keep track of how many CQEs we've read
        self.read_total += 1;

        // Result, user_data
        Ok((res, user_pid, user_smaps))
    }

    // Add a IO operation to the queue. *** THERE ARE NO CHECKS! ***
    pub fn add_to_queue(&mut self, user_data: (u32, u32), buffer: &mut Vec::<u8>, fd: i32, op: IOOPS)  {
        // Load current tail
        let mut tail: u32 = unsafe { &*self.sring_tail }.load(Ordering::Acquire);

        // Index of SQE entry
        let index: u32 = unsafe { tail & (*self.sring_mask) };

        // Get SQE entry
        let sqe = unsafe { &mut *self.sqes.offset(index as isize) };

        // Set the options for our request
        sqe.opcode = op as u8;
        sqe.fd = fd;
        sqe.addr = buffer.as_mut_ptr() as u64;
        sqe.len = buffer.capacity() as u32;
        //sqe.user_data = user_data;
        sqe.user_pid = user_data.0;
        sqe.user_smaps = user_data.1;

        // Update array
        unsafe { *self.sring_array.offset(index as isize) = index};

        // Increase tail and save it
        tail += 1;
        unsafe { &*self.sring_tail }.store(tail, Ordering::Release);

        // Add to the number to submit on the next system call
        self.submit += 1;
    }

    // Spin until the next result is available, in my case it should be instantly
    pub fn spin_next(&mut self) -> Result<(i32, u32, u32), UringError> {
        loop {
            let result = self.read_from_cq();

            match result {
                Ok(_) => {
                    return result;
                },

                Err(UringError::ReadFromCqEmptyBuffer) => {
                    if self.read_total == self.submit_total {
                        return Err(UringError::JobComplete);
                    }
                },

                _ => return result,
            }
        }
    }

    // Submit all queued operations to the kernel
    pub fn submit_all(&mut self) -> Result<i32, UringError> {
        let ret: i32;
        unsafe {
            asm!("syscall",
                in("rax") 426, // SYS_IO_URING_ENTER
                in("rdi") self.ring_fd, // io_uring fd
                in("rsi") self.submit, // to_submit
                in("rdx") 0, // min_complete
                in("r10") 0, // flags
                in("r8") 0, // sigset_t
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret,
            );
        }

        // If it's negative something has gone horrible wrong
        checkerr!(ret.is_negative(), UringError::SubmitToSqResult(ret));

        // It should return the same number as we submitted
        checkerr!(ret as u32 != self.submit, UringError::SubmitToSqResult(ret));

        // Reset amount to submit to zero
        self.submit = 0;

        // Save total amount submitted
        self.submit_total += ret as u64;

        Ok(ret)
    }

    // Wait for all operations to complete
    pub fn _wait_for_all(&mut self) -> Result<i32, UringError> {
        let ret: i32;
        unsafe {
            asm!("syscall",
                in("rax") 426, // SYS_IO_URING_ENTER
                in("rdi") self.ring_fd, // io_uring fd
                in("rsi") 0, // to_submit
                in("rdx") self.submit_total - self.read_total, // min_complete
                in("r10") 1u32 << 0, // flags, 1u32 << 0 == IORING_ENTER_GETEVENTS
                in("r8") 0, // sigset_t
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret,
            );
        }

        checkerr!(ret.is_negative(), UringError::SubmitToSqResult(ret));

        Ok(ret)
    }

    // Reset counts to zero
    pub fn reset(&mut self) {
        self.read_total = 0;
        self.submit_total = 0;
        self.submit = 0;
    }
}

// Close the io_uring fd when it goes out of scope
// The memory mmap is freed automatically
impl Drop for Uring {
    fn drop(&mut self) {
        if self.ring_fd > 0 {
            unsafe {
                asm!("syscall",
                    in("rax") 3, // SYS_CLOSE
                    in("rdi") self.ring_fd,
                    out("rcx") _,
                    out("r11") _,
                    lateout("rax") _,
                );
            }
        }
    }
}

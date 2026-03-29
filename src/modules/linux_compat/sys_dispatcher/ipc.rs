use super::super::*;
use super::SyscallDispFrame;
use crate::hal::syscalls_consts::linux_nr;

pub fn dispatch_ipc(nr: usize, f: &mut SyscallDispFrame) -> Option<usize> {
    match nr {
        linux_nr::SHMGET => Some(sys_linux_shmget(f.a1 as i32, f.a2, f.a3 as i32)),
        linux_nr::SHMAT => Some(sys_linux_shmat(f.a1 as i32, f.u2(), f.a3 as i32)),
        linux_nr::SHMDT => Some(sys_linux_shmdt(f.u1())),
        linux_nr::SHMCTL => Some(sys_linux_shmctl(f.a1 as i32, f.a2 as i32, f.u3())),

        // System V Semaphores
        linux_nr::SEMGET => Some(sys_linux_semget(f.a1 as i32, f.a2 as i32, f.a3 as i32)),
        linux_nr::SEMOP => Some(sys_linux_semop(f.a1 as i32, f.u2(), f.a3)),
        linux_nr::SEMCTL => Some(sys_linux_semctl(
            f.a1 as i32,
            f.a2 as i32,
            f.a3 as i32,
            f.a4,
        )),

        // System V Message Queues
        linux_nr::MSGGET => Some(sys_linux_msgget(f.a1 as i32, f.a2 as i32)),
        linux_nr::MSGSND => Some(sys_linux_msgsnd(f.a1 as i32, f.u2(), f.a3, f.a4 as i32)),
        linux_nr::MSGRCV => Some(sys_linux_msgrcv(
            f.a1 as i32,
            f.u2(),
            f.a3,
            f.a4 as i64,
            f.a5 as i32,
        )),

        // POSIX Message Queues
        linux_nr::MQ_OPEN => Some(sys_linux_mq_open(f.u1(), f.a2 as i32, f.a3 as u32, f.u4())),
        linux_nr::MQ_UNLINK => Some(sys_linux_mq_unlink(f.u1())),
        linux_nr::MQ_TIMEDSEND => Some(sys_linux_mq_timedsend(
            f.fd1(),
            f.u2(),
            f.a3,
            f.a4 as u32,
            f.u5(),
        )),
        linux_nr::MQ_TIMEDRECEIVE => Some(sys_linux_mq_timedreceive(
            f.fd1(),
            f.u2(),
            f.a3,
            f.u4(),
            f.u5(),
        )),
        linux_nr::MQ_GETSETATTR => Some(sys_linux_mq_getsetattr(f.fd1(), f.u2(), f.u3())),

        _ => None,
    }
}

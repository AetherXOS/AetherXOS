use super::*;

fn test_auxv(entry: usize) -> [ExecveAuxEntry<'static>; 2] {
    [
        ExecveAuxEntry {
            key: EXECVE_AUXV_AT_ENTRY,
            value: ExecveAuxValue::Word(entry),
        },
        ExecveAuxEntry {
            key: EXECVE_AUXV_AT_PAGESZ,
            value: ExecveAuxValue::Word(crate::interfaces::memory::PAGE_SIZE_4K as usize),
        },
    ]
}

fn read_user_word(ptr: usize) -> usize {
    with_user_read_bytes(ptr, core::mem::size_of::<usize>(), |src| {
        let mut tmp = [0u8; core::mem::size_of::<usize>()];
        tmp.copy_from_slice(src);
        usize::from_ne_bytes(tmp)
    })
    .expect("user word")
}

fn read_user_c_string_at(ptr: usize, max_len: usize) -> alloc::string::String {
    with_user_read_bytes(ptr, max_len, |src| {
        let len = src.iter().position(|&b| b == 0).unwrap_or(max_len);
        alloc::string::String::from_utf8(src[..len].to_vec()).expect("utf8")
    })
    .expect("user string")
}

fn stack_word_at(sp: usize, index: usize) -> usize {
    let word_size = core::mem::size_of::<usize>();
    read_user_word(sp + (index * word_size))
}

#[test_case]
fn execve_stack_required_bytes_accounts_for_strings_and_pointers() {
    let argv = alloc::vec![
        alloc::string::String::from("/bin/app"),
        alloc::string::String::from("--flag")
    ];
    let envp = alloc::vec![alloc::string::String::from("HOME=/")];
    let required = execve_stack_required_bytes(&argv, &envp, &test_auxv(0x4444)).expect("size");
    assert!(required >= 64);
    assert!(required >= argv.iter().map(|s| s.len() + 1).sum::<usize>());
}

#[test_case]
fn push_execve_user_c_string_writes_string_and_trailing_nul() {
    let mut stack = [0u8; 64];
    let stack_end = stack.as_mut_ptr() as u64 + stack.len() as u64;
    let mut sp = stack_end;
    let ptr = push_execve_user_c_string(&mut sp, "abc").expect("push string");
    assert_eq!(read_user_c_string_at(ptr, 8), "abc");
    assert_eq!(
        with_user_read_bytes(ptr, 4, |src| src[3]).expect("nul byte"),
        0
    );
}

#[test_case]
fn prepare_execve_user_stack_rejects_too_small_stack() {
    let argv = alloc::vec![alloc::string::String::from("prog")];
    let envp = alloc::vec![alloc::string::String::from("KEY=VALUE")];
    let auxv = test_auxv(0x1234);
    let required = execve_stack_required_bytes(&argv, &envp, &auxv).expect("size");
    assert_eq!(
        prepare_execve_user_stack(0x1000, (required - 1) as u64, &argv, &envp, &auxv),
        Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
    );
}

#[test_case]
fn prepare_execve_user_stack_places_argc_at_stack_top_word() {
    let argv = alloc::vec![
        alloc::string::String::from("prog"),
        alloc::string::String::from("arg1")
    ];
    let envp = alloc::vec![alloc::string::String::from("ENV=1")];
    let mut stack = [0u8; 512];
    let stack_start = stack.as_mut_ptr() as u64;

    let auxv = test_auxv(0x4444);
    let sp = prepare_execve_user_stack(stack_start, stack.len() as u64, &argv, &envp, &auxv)
        .expect("stack prepared");
    assert_eq!(sp as usize % core::mem::size_of::<usize>(), 0);

    let argc = read_user_word(sp as usize);
    assert_eq!(argc, argv.len());
}

#[test_case]
fn prepare_execve_user_stack_writes_argv_and_envp_strings() {
    let argv = alloc::vec![
        alloc::string::String::from("prog"),
        alloc::string::String::from("arg1")
    ];
    let envp = alloc::vec![
        alloc::string::String::from("ENV=1"),
        alloc::string::String::from("HOME=/root"),
    ];
    let mut stack = [0u8; 768];
    let stack_start = stack.as_mut_ptr() as u64;

    let auxv = test_auxv(0x4444);
    let sp = prepare_execve_user_stack(stack_start, stack.len() as u64, &argv, &envp, &auxv)
        .expect("stack prepared");

    let argc = stack_word_at(sp as usize, 0);
    assert_eq!(argc, argv.len());

    let (_, envp_null_index, first_argv_index) =
        execve_stack_layout_indexes(argv.len(), envp.len(), auxv.len());
    let env_null = stack_word_at(sp as usize, envp_null_index);
    let env0_ptr = stack_word_at(sp as usize, envp_null_index + envp.len());
    let env1_ptr = stack_word_at(sp as usize, envp_null_index + envp.len() - 1);
    let argv_null = stack_word_at(sp as usize, envp_null_index + envp.len() + 1);
    let argv0_ptr = stack_word_at(sp as usize, first_argv_index);
    let argv1_ptr = stack_word_at(sp as usize, first_argv_index - 1);

    assert_eq!(env_null, 0);
    assert_eq!(read_user_c_string_at(argv0_ptr, 16), "prog");
    assert_eq!(read_user_c_string_at(argv1_ptr, 16), "arg1");
    assert_eq!(argv_null, 0);
    assert_eq!(read_user_c_string_at(env0_ptr, 16), "ENV=1");
    assert_eq!(read_user_c_string_at(env1_ptr, 32), "HOME=/root");
}

#[test_case]
fn prepare_execve_user_stack_writes_auxv_entry_and_null_terminator() {
    let argv = alloc::vec![alloc::string::String::from("prog")];
    let envp = alloc::vec![alloc::string::String::from("ENV=1")];
    let mut stack = [0u8; 512];
    let stack_start = stack.as_mut_ptr() as u64;
    let entry = 0xDEAD_BEEFusize;

    let auxv = test_auxv(entry);
    let sp = prepare_execve_user_stack(stack_start, stack.len() as u64, &argv, &envp, &auxv)
        .expect("stack prepared");

    let (aux_entry_index, _, _) =
        execve_stack_layout_indexes(argv.len(), envp.len(), auxv.len());
    let aux_entry_type = stack_word_at(sp as usize, aux_entry_index);
    let aux_entry_value = stack_word_at(sp as usize, aux_entry_index + 1);
    let aux_pagesz_type = stack_word_at(sp as usize, aux_entry_index + 2);
    let aux_pagesz_value = stack_word_at(sp as usize, aux_entry_index + 3);
    let aux_null_type = stack_word_at(sp as usize, aux_entry_index + 4);
    let aux_null_value = stack_word_at(sp as usize, aux_entry_index + 5);

    assert_eq!(aux_entry_type, EXECVE_AUXV_AT_ENTRY);
    assert_eq!(aux_entry_value, entry);
    assert_eq!(aux_pagesz_type, EXECVE_AUXV_AT_PAGESZ);
    assert_eq!(aux_pagesz_value, crate::interfaces::memory::PAGE_SIZE_4K);
    assert_eq!(aux_null_type, EXECVE_AUXV_AT_NULL);
    assert_eq!(aux_null_value, 0);
}

#[test_case]
fn prepare_execve_user_stack_writes_string_backed_auxv_entries() {
    let argv = alloc::vec![alloc::string::String::from("prog")];
    let envp = alloc::vec![];
    let mut stack = [0u8; 512];
    let stack_start = stack.as_mut_ptr() as u64;
    let auxv = [
        ExecveAuxEntry {
            key: EXECVE_AUXV_AT_ENTRY,
            value: ExecveAuxValue::Word(0x1234),
        },
        ExecveAuxEntry {
            key: EXECVE_AUXV_AT_EXECFN,
            value: ExecveAuxValue::CString("/bin/prog"),
        },
    ];

    let sp = prepare_execve_user_stack(stack_start, stack.len() as u64, &argv, &envp, &auxv)
        .expect("stack prepared");
    let (aux_entry_index, _, _) =
        execve_stack_layout_indexes(argv.len(), envp.len(), auxv.len());
    let execfn_type = stack_word_at(sp as usize, aux_entry_index + 2);
    let execfn_ptr = stack_word_at(sp as usize, aux_entry_index + 3);

    assert_eq!(execfn_type, EXECVE_AUXV_AT_EXECFN);
    assert_eq!(read_user_c_string_at(execfn_ptr, 16), "/bin/prog");
}

#[test_case]
fn prepare_execve_user_stack_writes_binary_backed_auxv_entries() {
    let argv = alloc::vec![alloc::string::String::from("prog")];
    let envp = alloc::vec![];
    let mut stack = [0u8; 512];
    let stack_start = stack.as_mut_ptr() as u64;
    let random = [0x11u8, 0x22, 0x33, 0x44];
    let auxv = [
        ExecveAuxEntry {
            key: EXECVE_AUXV_AT_ENTRY,
            value: ExecveAuxValue::Word(0x1234),
        },
        ExecveAuxEntry {
            key: 25,
            value: ExecveAuxValue::Bytes(&random),
        },
    ];

    let sp = prepare_execve_user_stack(stack_start, stack.len() as u64, &argv, &envp, &auxv)
        .expect("stack prepared");
    let (aux_entry_index, _, _) =
        execve_stack_layout_indexes(argv.len(), envp.len(), auxv.len());
    let random_type = stack_word_at(sp as usize, aux_entry_index + 2);
    let random_ptr = stack_word_at(sp as usize, aux_entry_index + 3);

    assert_eq!(random_type, 25);
    let copied = with_user_read_bytes(random_ptr, random.len(), |src| src.to_vec())
        .expect("random bytes");
    assert_eq!(copied.as_slice(), random.as_slice());
}

#[test_case]
fn execve_stack_layout_indexes_handle_empty_vectors() {
    let (aux_entry_index, envp_null_index, first_argv_index) =
        execve_stack_layout_indexes(0, 0, test_auxv(0).len());
    assert_eq!(aux_entry_index, 1);
    assert_eq!(envp_null_index, 7);
    assert_eq!(first_argv_index, 8);
}

#[test_case]
fn execve_stack_pointer_indexes_track_argv_and_envp_positions() {
    let auxv_len = test_auxv(0).len();
    let (argv0_index, env0_index) = execve_stack_pointer_indexes(3, 2, auxv_len, 0, 0);
    let (argv2_index, env1_index) = execve_stack_pointer_indexes(3, 2, auxv_len, 2, 1);
    assert_eq!(argv0_index, 12);
    assert_eq!(argv2_index, 10);
    assert_eq!(env0_index, 9);
    assert_eq!(env1_index, 8);
}

#[test_case]
fn prepare_execve_user_stack_handles_empty_argv_and_envp() {
    let argv = alloc::vec![];
    let envp = alloc::vec![];
    let mut stack = [0u8; 256];
    let stack_start = stack.as_mut_ptr() as u64;

    let auxv = test_auxv(0x1111);
    let sp = prepare_execve_user_stack(stack_start, stack.len() as u64, &argv, &envp, &auxv)
        .expect("stack prepared");

    assert_eq!(stack_word_at(sp as usize, 0), 0);
    let (_, envp_null_index, _) = execve_stack_layout_indexes(0, 0, auxv.len());
    assert_eq!(stack_word_at(sp as usize, envp_null_index), 0);
    assert_eq!(stack_word_at(sp as usize, envp_null_index + 1), 0);
}

#[test_case]
fn prepare_execve_user_stack_keeps_pointer_order_for_longer_vectors() {
    let argv = alloc::vec![
        alloc::string::String::from("prog"),
        alloc::string::String::from("--alpha"),
        alloc::string::String::from("--beta"),
    ];
    let envp = alloc::vec![
        alloc::string::String::from("HOME=/tmp"),
        alloc::string::String::from("TERM=xterm"),
    ];
    let mut stack = [0u8; 1024];
    let stack_start = stack.as_mut_ptr() as u64;

    let auxv = test_auxv(0x2222);
    let sp = prepare_execve_user_stack(stack_start, stack.len() as u64, &argv, &envp, &auxv)
        .expect("stack prepared");

    let (argv0_index, env0_index) =
        execve_stack_pointer_indexes(argv.len(), envp.len(), auxv.len(), 0, 0);
    let (argv1_index, env1_index) =
        execve_stack_pointer_indexes(argv.len(), envp.len(), auxv.len(), 1, 1);
    let (argv2_index, _) =
        execve_stack_pointer_indexes(argv.len(), envp.len(), auxv.len(), 2, 0);

    let argv0_ptr = stack_word_at(sp as usize, argv0_index);
    let argv1_ptr = stack_word_at(sp as usize, argv1_index);
    let argv2_ptr = stack_word_at(sp as usize, argv2_index);
    let env0_ptr = stack_word_at(sp as usize, env0_index);
    let env1_ptr = stack_word_at(sp as usize, env1_index);

    assert_eq!(read_user_c_string_at(argv0_ptr, 16), "prog");
    assert_eq!(read_user_c_string_at(argv1_ptr, 16), "--alpha");
    assert_eq!(read_user_c_string_at(argv2_ptr, 16), "--beta");
    assert_eq!(read_user_c_string_at(env0_ptr, 16), "HOME=/tmp");
    assert_eq!(read_user_c_string_at(env1_ptr, 16), "TERM=xterm");
    assert!(argv0_ptr < argv1_ptr);
    assert!(argv1_ptr < argv2_ptr);
    assert!(env0_ptr < env1_ptr);
}

#[test_case]
fn prepare_execve_user_stack_writes_null_terminators_after_vectors() {
    let argv = alloc::vec![alloc::string::String::from("prog")];
    let envp = alloc::vec![
        alloc::string::String::from("A=1"),
        alloc::string::String::from("B=2"),
    ];
    let mut stack = [0u8; 512];
    let stack_start = stack.as_mut_ptr() as u64;

    let auxv = test_auxv(0x3333);
    let sp = prepare_execve_user_stack(stack_start, stack.len() as u64, &argv, &envp, &auxv)
        .expect("stack prepared");
    let (_, envp_null_index, first_argv_index) =
        execve_stack_layout_indexes(argv.len(), envp.len(), auxv.len());

    assert_eq!(stack_word_at(sp as usize, envp_null_index), 0);
    assert_eq!(stack_word_at(sp as usize, first_argv_index - argv.len()), 0);
}

#[test_case]
fn prepare_execve_user_stack_keeps_auxv_before_envp_and_argv_tables() {
    let argv = alloc::vec![
        alloc::string::String::from("prog"),
        alloc::string::String::from("arg")
    ];
    let envp = alloc::vec![alloc::string::String::from("LANG=C")];
    let mut stack = [0u8; 512];
    let stack_start = stack.as_mut_ptr() as u64;

    let auxv = test_auxv(0xABCD);
    let sp = prepare_execve_user_stack(stack_start, stack.len() as u64, &argv, &envp, &auxv)
        .expect("stack prepared");
    let (aux_entry_index, envp_null_index, first_argv_index) =
        execve_stack_layout_indexes(argv.len(), envp.len(), auxv.len());

    assert_eq!(
        stack_word_at(sp as usize, aux_entry_index),
        EXECVE_AUXV_AT_ENTRY
    );
    assert_eq!(stack_word_at(sp as usize, envp_null_index), 0);
    assert!(aux_entry_index < envp_null_index);
    assert!(envp_null_index < first_argv_index);
}

#[test_case]
fn prepare_execve_user_stack_rejects_auxv_entries_containing_at_null_key() {
    let argv = alloc::vec![alloc::string::String::from("prog")];
    let envp = alloc::vec![];
    let mut stack = [0u8; 256];
    let stack_start = stack.as_mut_ptr() as u64;
    let auxv = [ExecveAuxEntry {
        key: EXECVE_AUXV_AT_NULL,
        value: ExecveAuxValue::Word(0),
    }];

    let rc = prepare_execve_user_stack(stack_start, stack.len() as u64, &argv, &envp, &auxv);
    assert_eq!(rc, Err(linux_errno(crate::modules::posix_consts::errno::EINVAL)));
}

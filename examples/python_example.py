#!/usr/bin/env python3
"""
Example: Interactive Memory Scanner

This example demonstrates how to use the memscan Python bindings to perform
interactive memory scanning on a target process.
"""

import memscan
import time


def find_value_example():
    """Example: Find and modify a specific value in a process"""
    
    # Find process by name (or use a PID directly)
    process_name = input("Enter process name (e.g., 'notepad'): ")
    pid = memscan.find_process_by_name(process_name)
    
    if pid is None:
        print(f"Process '{process_name}' not found")
        return
    
    print(f"Found process: PID={pid}")
    
    # Open the process
    proc = memscan.open_process(pid)
    print("Process opened successfully")
    
    # Get system information
    sys_info = memscan.query_system_info()
    print(f"System info: {sys_info}")
    
    # Get module regions
    modules = memscan.get_process_module_regions(proc)
    print(f"Found {len(modules)} module regions")
    
    # Create an interactive scanner
    value_type = input("Enter value type (i8/i16/i32/i64/u8/u16/u32/u64/f32/f64) [default: i32]: ").strip() or "i32"
    scanner = memscan.create_interactive_scanner(proc, modules, value_type)
    print(f"Scanner created for value type: {value_type}")
    
    # Perform initial scan
    print("\nPerforming initial scan...")
    count = scanner.initial_scan()
    print(f"Found {count} possible addresses")
    
    # Interactive filtering
    while True:
        print(f"\nCurrent matches: {scanner.match_count()}")
        print("\nAvailable operations:")
        print("  1. Filter by exact value (eq)")
        print("  2. Filter by less than (lt)")
        print("  3. Filter by greater than (gt)")
        print("  4. Filter by increased")
        print("  5. Filter by decreased")
        print("  6. Filter by changed")
        print("  7. Filter by unchanged")
        print("  8. List matches (first 20)")
        print("  9. Set value at all matches")
        print("  10. Save checkpoint")
        print("  11. List checkpoints")
        print("  0. Exit")
        
        choice = input("\nEnter choice: ").strip()
        
        if choice == "0":
            break
        elif choice == "1":
            value = float(input("Enter value: "))
            count = scanner.filter_eq(value)
            print(f"Filtered to {count} addresses")
        elif choice == "2":
            value = float(input("Enter value: "))
            count = scanner.filter_lt(value)
            print(f"Filtered to {count} addresses")
        elif choice == "3":
            value = float(input("Enter value: "))
            count = scanner.filter_gt(value)
            print(f"Filtered to {count} addresses")
        elif choice == "4":
            count = scanner.filter_increased()
            print(f"Filtered to {count} addresses")
        elif choice == "5":
            count = scanner.filter_decreased()
            print(f"Filtered to {count} addresses")
        elif choice == "6":
            count = scanner.filter_changed()
            print(f"Filtered to {count} addresses")
        elif choice == "7":
            count = scanner.filter_unchanged()
            print(f"Filtered to {count} addresses")
        elif choice == "8":
            matches = scanner.get_matches()
            print(f"\nShowing first 20 of {len(matches)} matches:")
            for i, match in enumerate(matches[:20]):
                print(f"  {i}: {match}")
        elif choice == "9":
            value = float(input("Enter value to set: "))
            count = scanner.set_value(value)
            print(f"Set value at {count} addresses")
        elif choice == "10":
            name = input("Enter checkpoint name: ")
            scanner.save_checkpoint(name)
            print(f"Checkpoint '{name}' saved")
        elif choice == "11":
            checkpoints = scanner.list_checkpoints()
            print(f"Saved checkpoints: {checkpoints}")
        else:
            print("Invalid choice")


def pattern_scan_example():
    """Example: Scan for a byte pattern in memory"""
    
    # Find process
    process_name = input("Enter process name: ")
    pid = memscan.find_process_by_name(process_name)
    
    if pid is None:
        print(f"Process '{process_name}' not found")
        return
    
    print(f"Found process: PID={pid}")
    
    # Open the process
    proc = memscan.open_process(pid)
    print("Process opened successfully")
    
    # Get a memory address to read
    address_str = input("Enter memory address (hex, e.g., 0x1000): ")
    address = int(address_str, 16)
    
    size = int(input("Enter number of bytes to read: "))
    
    # Read memory
    try:
        data = memscan.read_process_memory(proc, address, size)
        print(f"Read {len(data)} bytes:")
        print(" ".join(f"{b:02x}" for b in data))
    except Exception as e:
        print(f"Failed to read memory: {e}")


def main():
    """Main entry point"""
    print("=== MemScan Python Bindings Example ===\n")
    print("Choose an example:")
    print("  1. Interactive memory scanner")
    print("  2. Pattern scan / memory read")
    
    choice = input("\nEnter choice: ").strip()
    
    if choice == "1":
        find_value_example()
    elif choice == "2":
        pattern_scan_example()
    else:
        print("Invalid choice")


if __name__ == "__main__":
    main()

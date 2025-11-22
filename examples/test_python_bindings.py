#!/usr/bin/env python3
"""
Simple test script to verify Python bindings are working correctly.
This tests basic functionality without requiring a running process.
"""

import sys

def test_imports():
    """Test that we can import the memscan module"""
    print("Testing imports...")
    try:
        import memscan
        print("✓ Successfully imported memscan module")
        return True
    except ImportError as e:
        print(f"✗ Failed to import memscan: {e}")
        return False

def test_module_exports():
    """Test that expected functions and classes are exported"""
    print("\nTesting module exports...")
    import memscan
    
    expected_functions = [
        'open_process',
        'find_process_by_name',
        'query_system_info',
        'get_process_module_regions',
        'parse_hex_pattern',
        'read_process_memory',
        'write_process_memory',
        'create_interactive_scanner',
    ]
    
    expected_classes = [
        'PyProcessHandle',
        'PyMemoryRegion',
        'PySystemInfo',
        'PyInteractiveScanner',
        'PyMatchedAddress',
    ]
    
    all_passed = True
    
    for func_name in expected_functions:
        if hasattr(memscan, func_name):
            print(f"✓ Function '{func_name}' is exported")
        else:
            print(f"✗ Function '{func_name}' is NOT exported")
            all_passed = False
    
    for class_name in expected_classes:
        if hasattr(memscan, class_name):
            print(f"✓ Class '{class_name}' is exported")
        else:
            print(f"✗ Class '{class_name}' is NOT exported")
            all_passed = False
    
    return all_passed

def test_system_info():
    """Test getting system information"""
    print("\nTesting system info...")
    import memscan
    
    try:
        sys_info = memscan.query_system_info()
        print(f"✓ Got system info: {sys_info}")
        
        # Check that attributes exist
        attrs = ['min_app_addr', 'max_app_addr', 'page_size', 'granularity']
        for attr in attrs:
            if hasattr(sys_info, attr):
                value = getattr(sys_info, attr)
                print(f"  - {attr}: 0x{value:x}" if 'addr' in attr else f"  - {attr}: {value}")
            else:
                print(f"✗ Missing attribute '{attr}'")
                return False
        
        return True
    except Exception as e:
        print(f"✗ Failed to get system info: {e}")
        return False

def test_hex_pattern_parsing():
    """Test hex pattern parsing"""
    print("\nTesting hex pattern parsing...")
    import memscan
    
    test_cases = [
        ("4D 5A 90 00", [0x4D, 0x5A, 0x90, 0x00]),
        ("DEADBEEF", [0xDE, 0xAD, 0xBE, 0xEF]),
        ("de ad be ef", [0xDE, 0xAD, 0xBE, 0xEF]),
    ]
    
    all_passed = True
    for pattern, expected in test_cases:
        try:
            result = memscan.parse_hex_pattern(pattern)
            if list(result) == expected:
                print(f"✓ Pattern '{pattern}' parsed correctly")
            else:
                print(f"✗ Pattern '{pattern}' parsed incorrectly: got {list(result)}, expected {expected}")
                all_passed = False
        except Exception as e:
            print(f"✗ Failed to parse '{pattern}': {e}")
            all_passed = False
    
    # Test invalid patterns
    invalid_patterns = ["ABC", "ABGH"]
    for pattern in invalid_patterns:
        try:
            result = memscan.parse_hex_pattern(pattern)
            print(f"✗ Pattern '{pattern}' should have failed but didn't")
            all_passed = False
        except Exception:
            print(f"✓ Pattern '{pattern}' correctly rejected")
    
    return all_passed

def test_find_process():
    """Test finding a process (may not find anything, but shouldn't crash)"""
    print("\nTesting process finding...")
    import memscan
    
    try:
        # Try to find a common process (this may or may not exist)
        pid = memscan.find_process_by_name("init")
        if pid is None:
            print("✓ find_process_by_name returned None (process not found)")
        else:
            print(f"✓ find_process_by_name found process with PID: {pid}")
        return True
    except Exception as e:
        print(f"✗ find_process_by_name raised exception: {e}")
        return False

def main():
    """Run all tests"""
    print("=" * 60)
    print("MemScan Python Bindings Test Suite")
    print("=" * 60)
    
    tests = [
        ("Import", test_imports),
        ("Module Exports", test_module_exports),
        ("System Info", test_system_info),
        ("Hex Pattern Parsing", test_hex_pattern_parsing),
        ("Process Finding", test_find_process),
    ]
    
    results = []
    for test_name, test_func in tests:
        try:
            passed = test_func()
            results.append((test_name, passed))
        except Exception as e:
            print(f"\n✗ Test '{test_name}' crashed: {e}")
            import traceback
            traceback.print_exc()
            results.append((test_name, False))
    
    # Print summary
    print("\n" + "=" * 60)
    print("Test Summary")
    print("=" * 60)
    
    passed_count = sum(1 for _, passed in results if passed)
    total_count = len(results)
    
    for test_name, passed in results:
        status = "✓ PASS" if passed else "✗ FAIL"
        print(f"{status}: {test_name}")
    
    print(f"\nTotal: {passed_count}/{total_count} tests passed")
    
    if passed_count == total_count:
        print("\n🎉 All tests passed!")
        return 0
    else:
        print(f"\n❌ {total_count - passed_count} test(s) failed")
        return 1

if __name__ == "__main__":
    sys.exit(main())

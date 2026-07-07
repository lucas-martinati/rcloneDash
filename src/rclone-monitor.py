#!/usr/bin/env python3
"""RcloneDash — point d'entrée."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from backend.server import main
if __name__ == "__main__":
    main()

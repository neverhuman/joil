"""Keep imported protocol and render-boundary controls in ordinary software CI.

All fixtures are synthetic. Renderer tests execute a harmless stand-in, never Blender.
"""
from analysis.validation.v5.tests.test_protocol import ProtocolTests
from render.blender.test_plate import PlatePreflightTests

__all__ = ["ProtocolTests", "PlatePreflightTests"]

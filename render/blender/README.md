# Blender adapter boundary

This directory is a separately packaged GPL adapter. Core scene validation, scalar mapping,
evidence qualification, and cache identity live in `crucible-render` and do not import Blender.
The adapter never assumes Blender 5.2 LTS exists: `plate.py --probe` reports the executable and
version actually present; dry-run works without Blender. A real render requires a caller-supplied
exact build identity and a validated scene bundle. No scientific mesh smoothing or denoising is
enabled. Labels and legends are composited as a separate overlay layer. Camera, light and seed
defaults are versioned in `profile.json`.

The image is a presentation artifact. Its manifest preserves source evidence class and limitations;
rendering cannot upgrade them. Unavailable evidence produces an unavailable card only.

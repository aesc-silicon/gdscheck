# SPDX-FileCopyrightText: 2026 aesc silicon
# SPDX-License-Identifier: AGPL-3.0-or-later

# Keep the PDK macro unchanged, including its deep mode. Flatten only a copy of
# the selected input cell, so both reports use the same top-cell coordinates.
layout = RBA::Layout.new
layout.read($input)
top = layout.cell($topcell)
raise "No cell named #{$topcell}" unless top
top.flatten(true)
options = RBA::SaveLayoutOptions.new
options.add_cell(top.cell_index)
layout.write($output, options)

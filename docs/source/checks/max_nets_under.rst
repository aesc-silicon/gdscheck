.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

max_nets_under
==============

Net-aware. How many different nets one marker may cover: the regions of ``layers[1]``
under each region of ``layers[0]`` are resolved to nets, and a marker covering more than
``value`` of them is reported.


Semantics
---------

GF180's NAT.6 is the rule this exists for — "two or more COMPs at different potential are
not allowed under the same NAT layer". Upstream builds the different-potential pairs with
a net-aware spacing helper and asks which markers touch them; counting the nets under the
marker is what the rule says, and reaches the same answer.

Each conductor region is placed under whichever marker region holds its marker point. A
region whose net does not resolve counts as a net of its own, so an unresolved lookup can
only ever cost a false positive, never a false clean.


Layers
------

- ``layers[0]`` — the marker.
- ``layers[1]`` — what lies beneath it and is counted.


Parameters
----------

``net_of`` (a layer param)
   Optional. The conductor the nets are looked up on, when what is counted is a derived
   layer — NAT.6 counts the active *under the marker*, an intersection, which is in no
   connect graph. Defaults to ``layers[1]``.


Violation markers
-----------------

One point marker per marker region over the limit, at its marker.


KLayout equivalent
------------------

None directly; upstream's construction through a net-aware spacing helper.


Example
-------

.. code-block:: yaml

    - id: NAT.6
      check: max_nets_under
      layers: [nat, nat6_comp]
      value: 1.0
      layer_params:
        net_of: comp

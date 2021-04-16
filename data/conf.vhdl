configuration CFG_FULLADDER of
FULLADDER is
  for STRUCT
    for MODULE2: HALFADDER
       use entity work.HA2(GATE);
        port map (U => A,
                  V => B,
                  X => SUM,
                  Y => CARRY);
    end for;
 
    for others: HALFADDER
       use entity work.HA1(RTL);
    end for;
  end for;
end CFG_FULLADDER;
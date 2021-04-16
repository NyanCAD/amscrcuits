entity PARITY is
  generic (N : integer);
  port    (A : in std_logic_vector(N-1 downto 0);
         ODD : out std_ulogic);
end PARITY;
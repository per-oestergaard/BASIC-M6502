10 REM Array Test Program
20 REM Tests array declarations and operations
30 PRINT "ARRAY TESTS"
40 PRINT "==========="
50 REM Dimension array
60 DIM A(10)
70 REM Set array values
80 LET A(0) = 100
90 LET A(1) = 200
100 LET A(2) = 300
110 REM Print array values
120 PRINT A(0)
130 PRINT A(1)
140 PRINT A(2)
150 REM Array in loop
160 FOR I = 0 TO 2
170 PRINT A(I)
180 NEXT I
190 PRINT "END OF ARRAY TESTS"

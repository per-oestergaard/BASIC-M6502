10 REM Variable Assignment Test Program
20 REM Tests various variable assignment patterns
30 PRINT "VARIABLE TESTS"
40 PRINT "=============="
50 REM Single variable assignments
60 LET A = 10
70 PRINT A
80 LET B = 20
90 PRINT B
100 LET C = 30
110 PRINT C
120 REM Multiple assignments with different values
130 LET X = 100
140 LET Y = 200
150 LET Z = 300
160 PRINT X
170 PRINT Y
180 PRINT Z
190 REM Variable reassignment
200 LET A = 99
210 PRINT A
220 REM Using variables in expressions
230 LET RESULT = A+B+C
240 PRINT RESULT
250 REM Chained assignments
260 LET VAR1 = 1
270 LET VAR2 = VAR1
280 LET VAR3 = VAR2
290 PRINT VAR1
300 PRINT VAR2
310 PRINT VAR3
320 PRINT "END OF VARIABLE TESTS"

-- Solo para desarrollo. No crea cuentas ni contraseñas.
-- Registra estos estudiantes desde la interfaz eligiendo una contraseña.
INSERT INTO student_roster (student_code, full_name, email) VALUES
    ('20260001', 'Lucía Torres', 'lucia.torres@fisi.example'),
    ('20260002', 'Mateo Rojas', 'mateo.rojas@fisi.example')
ON CONFLICT DO NOTHING;
